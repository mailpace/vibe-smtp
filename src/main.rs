use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use reqwest::Client;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// SMTP server listen address
    #[arg(short, long, default_value = "127.0.0.1:2525")]
    listen: String,
    
    /// MailPace API endpoint
    #[arg(long, default_value = "https://app.mailpace.com/api/v1/send")]
    mailpace_endpoint: String,
    
    /// Default MailPace API token (optional, can be overridden by SMTP auth)
    #[arg(long, env = "MAILPACE_API_TOKEN")]
    default_mailpace_token: Option<String>,
    
    /// Enable TLS/STARTTLS support
    #[arg(long)]
    enable_tls: bool,
    
    /// Debug mode
    #[arg(short, long)]
    debug: bool,
}

#[derive(Serialize, Debug)]
struct MailPacePayload {
    from: String,
    to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    htmlbody: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    textbody: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replyto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    list_unsubscribe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<Attachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
}

#[derive(Serialize, Debug)]
struct Attachment {
    name: String,
    content: String,
    content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cid: Option<String>,
}

#[derive(Deserialize)]
struct MailPaceResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    errors: Option<Vec<String>>,
}

#[derive(Debug, PartialEq)]
enum SmtpState {
    Init,
    Helo,
    Mail,
    Rcpt,
    Data,
    Quit,
    StartTls, // New state for STARTTLS
}

enum Connection {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl Connection {
    async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()> {
        match self {
            Connection::Plain(stream) => stream.write_all(buf).await,
            Connection::Tls(stream) => stream.write_all(buf).await,
        }
    }
    
    async fn flush(&mut self) -> tokio::io::Result<()> {
        match self {
            Connection::Plain(stream) => stream.flush().await,
            Connection::Tls(stream) => stream.flush().await,
        }
    }
}

struct SmtpSession {
    client: Client,
    mailpace_endpoint: String,
    default_mailpace_token: Option<String>,
    state: SmtpState,
    helo: Option<String>,
    mail_from: Option<String>,
    rcpt_to: Vec<String>,
    data: Vec<u8>,
    auth_token: Option<String>, // Token provided via SMTP AUTH
    tls_acceptor: Option<TlsAcceptor>,
    supports_starttls: bool,
}

impl SmtpSession {
    fn new(client: Client, mailpace_endpoint: String, default_mailpace_token: Option<String>, tls_acceptor: Option<TlsAcceptor>) -> Self {
        let supports_starttls = tls_acceptor.is_some();
        Self {
            client,
            mailpace_endpoint,
            default_mailpace_token,
            state: SmtpState::Init,
            helo: None,
            mail_from: None,
            rcpt_to: Vec::new(),
            data: Vec::new(),
            auth_token: None,
            tls_acceptor,
            supports_starttls,
        }
    }

    async fn handle(&mut self, stream: TcpStream) -> Result<()> {
        let mut connection = Connection::Plain(stream);
        self.send_response(&mut connection, "220 vibe-gateway SMTP ready").await?;
        
        loop {
            let mut line = String::new();
            
            // Read command
            let command = match &mut connection {
                Connection::Plain(stream) => {
                    let mut reader = BufReader::new(stream);
                    let bytes_read = reader.read_line(&mut line).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    line.trim().to_string()
                }
                Connection::Tls(stream) => {
                    let mut reader = BufReader::new(stream);
                    let bytes_read = reader.read_line(&mut line).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    line.trim().to_string()
                }
            };
            
            debug!("Received command: {}", command);
            
            // Handle STARTTLS command specially
            if command.to_uppercase() == "STARTTLS" {
                if let Some(ref acceptor) = self.tls_acceptor {
                    self.send_response(&mut connection, "220 Ready to start TLS").await?;
                    
                    // Upgrade connection to TLS
                    if let Connection::Plain(plain_stream) = connection {
                        let tls_stream = acceptor.accept(plain_stream).await
                            .context("Failed to establish TLS connection")?;
                        connection = Connection::Tls(tls_stream);
                        info!("TLS connection established");
                    }
                    continue;
                } else {
                    self.send_response(&mut connection, "454 TLS not available").await?;
                    continue;
                }
            }
            
            // Process command
            match self.process_command(&command).await {
                Ok(response) => {
                    if let Some(resp) = response {
                        self.send_response(&mut connection, &resp).await?;
                    }
                }
                Err(e) => {
                    error!("Error processing command '{}': {}", command, e);
                    self.send_response(&mut connection, "451 Temporary local problem").await?;
                }
            }
            
            if self.state == SmtpState::Quit {
                break;
            }
            
            // Handle DATA command specially
            if self.state == SmtpState::Data {
                match &mut connection {
                    Connection::Plain(stream) => {
                        let mut reader = BufReader::new(stream);
                        if let Err(e) = self.read_data(&mut reader).await {
                            error!("Error reading data: {}", e);
                            self.send_response(&mut connection, "451 Error reading data").await?;
                        } else {
                            self.handle_data_processing(&mut connection).await?;
                        }
                    }
                    Connection::Tls(stream) => {
                        let mut reader = BufReader::new(stream);
                        if let Err(e) = self.read_data(&mut reader).await {
                            error!("Error reading data: {}", e);
                            self.send_response(&mut connection, "451 Error reading data").await?;
                        } else {
                            self.handle_data_processing(&mut connection).await?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn handle_data_processing(&mut self, connection: &mut Connection) -> Result<()> {
        match self.process_email_data().await {
            Ok(_) => {
                self.send_response(connection, "250 OK: Message accepted for delivery").await?;
            }
            Err(e) => {
                error!("Failed to send email to MailPace: {}", e);
                self.send_response(connection, &format!("550 Error: {}", e)).await?;
            }
        }
        self.reset_session();
        Ok(())
    }
    
    async fn process_command(&mut self, command: &str) -> Result<Option<String>> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }
        
        let cmd = parts[0].to_uppercase();
        
        match cmd.as_str() {
            "HELO" | "EHLO" => {
                if parts.len() > 1 {
                    self.helo = Some(parts[1].to_string());
                    self.state = SmtpState::Helo;
                    if cmd == "EHLO" {
                        let mut response = vec![
                            "250-vibe-gateway".to_string(),
                            "250-AUTH PLAIN LOGIN".to_string(),
                        ];
                        
                        if self.supports_starttls {
                            response.push("250-STARTTLS".to_string());
                        }
                        
                        response.push("250 8BITMIME".to_string());
                        Ok(Some(response.join("\r\n")))
                    } else {
                        Ok(Some("250 vibe-gateway".to_string()))
                    }
                } else {
                    Ok(Some("501 Syntax error in parameters".to_string()))
                }
            }
            "AUTH" => {
                if parts.len() >= 2 {
                    match parts[1].to_uppercase().as_str() {
                        "PLAIN" => {
                            if parts.len() >= 3 {
                                // AUTH PLAIN token - decode and extract token
                                if let Ok(decoded) = general_purpose::STANDARD.decode(parts[2]) {
                                    if let Ok(auth_string) = String::from_utf8(decoded) {
                                        // PLAIN format: \0username\0password
                                        // For MailPace, both username and password are the API token
                                        let parts: Vec<&str> = auth_string.split('\0').collect();
                                        if parts.len() >= 3 {
                                            self.auth_token = Some(parts[1].to_string()); // Use username as token
                                        }
                                    }
                                }
                                Ok(Some("235 Authentication successful".to_string()))
                            } else {
                                // Multi-step AUTH PLAIN
                                Ok(Some("334 ".to_string()))
                            }
                        }
                        "LOGIN" => {
                            // For LOGIN, we need to implement the multi-step process
                            // This is a simplified implementation that accepts any token
                            Ok(Some("334 VXNlcm5hbWU6".to_string())) // Username: in base64
                        }
                        _ => {
                            Ok(Some("504 Unrecognized authentication type".to_string()))
                        }
                    }
                } else {
                    Ok(Some("501 Syntax error in parameters".to_string()))
                }
            }
            "MAIL" => {
                if command.to_uppercase().starts_with("MAIL FROM:") {
                    let from = command[10..].trim();
                    let from = from.trim_start_matches('<').trim_end_matches('>');
                    self.mail_from = Some(from.to_string());
                    self.state = SmtpState::Mail;
                    Ok(Some("250 OK".to_string()))
                } else {
                    Ok(Some("501 Syntax error in parameters".to_string()))
                }
            }
            "RCPT" => {
                if command.to_uppercase().starts_with("RCPT TO:") {
                    let to = command[8..].trim();
                    let to = to.trim_start_matches('<').trim_end_matches('>');
                    self.rcpt_to.push(to.to_string());
                    self.state = SmtpState::Rcpt;
                    Ok(Some("250 OK".to_string()))
                } else {
                    Ok(Some("501 Syntax error in parameters".to_string()))
                }
            }
            "DATA" => {
                if self.mail_from.is_some() && !self.rcpt_to.is_empty() {
                    self.state = SmtpState::Data;
                    Ok(Some("354 End data with <CR><LF>.<CR><LF>".to_string()))
                } else {
                    Ok(Some("503 Bad sequence of commands".to_string()))
                }
            }
            "RSET" => {
                self.reset_session();
                Ok(Some("250 OK".to_string()))
            }
            "NOOP" => {
                Ok(Some("250 OK".to_string()))
            }
            "QUIT" => {
                self.state = SmtpState::Quit;
                Ok(Some("221 Goodbye".to_string()))
            }
            _ => {
                Ok(Some("502 Command not implemented".to_string()))
            }
        }
    }
    
    async fn read_data<R>(&mut self, reader: &mut BufReader<R>) -> Result<()> 
    where 
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut line = String::new();
        self.data.clear();
        
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break;
            }
            
            if line.trim() == "." {
                break;
            }
            
            // Handle dot stuffing
            if line.starts_with("..") {
                line.remove(0);
            }
            
            self.data.extend_from_slice(line.as_bytes());
        }
        
        Ok(())
    }
    
    async fn process_email_data(&mut self) -> Result<()> {
        let email_content = String::from_utf8_lossy(&self.data);
        debug!("Email content: {}", email_content);
        
        // Parse email manually - simplified approach
        let payload = self.parse_email_to_mailpace_payload(&email_content)?;
        
        // Send to MailPace API
        self.send_to_mailpace(&payload).await?;
        
        Ok(())
    }
    
    fn parse_email_to_mailpace_payload(&self, email_content: &str) -> Result<MailPacePayload> {
        let mut headers = HashMap::new();
        let mut body_lines = Vec::new();
        let mut in_headers = true;
        
        for line in email_content.lines() {
            if in_headers {
                if line.is_empty() {
                    in_headers = false;
                    continue;
                }
                
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_lowercase();
                    let value = line[colon_pos + 1..].trim().to_string();
                    headers.insert(key, value);
                }
            } else {
                body_lines.push(line);
            }
        }
        
        let from = self.mail_from.as_ref()
            .context("No sender address")?
            .clone();
        
        let to = self.rcpt_to.join(", ");
        
        let subject = headers.get("subject").cloned();
        let cc = headers.get("cc").cloned();
        let bcc = headers.get("bcc").cloned();
        let replyto = headers.get("reply-to").cloned();
        let list_unsubscribe = headers.get("x-list-unsubscribe")
            .or_else(|| headers.get("list-unsubscribe"))
            .cloned();
        
        let tags = headers.get("x-mailpace-tags")
            .map(|tags_str| {
                tags_str.split(',')
                    .map(|tag| tag.trim().to_string())
                    .collect::<Vec<_>>()
            });
        
        let body = body_lines.join("\n");
        
        // Simple HTML detection
        let (htmlbody, textbody) = if body.contains("<html>") || body.contains("<HTML>") {
            (Some(body), None)
        } else {
            (None, Some(body))
        };
        
        Ok(MailPacePayload {
            from,
            to,
            cc,
            bcc,
            subject,
            htmlbody,
            textbody,
            replyto,
            list_unsubscribe,
            attachments: None, // Simplified - no attachment support for now
            tags,
        })
    }
    
    async fn send_to_mailpace(&self, payload: &MailPacePayload) -> Result<()> {
        // Use token from SMTP AUTH first, then fall back to default
        let token = self.auth_token.as_ref()
            .or(self.default_mailpace_token.as_ref())
            .context("No MailPace API token provided via SMTP AUTH or default configuration")?;
        
        debug!("Sending payload to MailPace: {:?}", payload);
        
        let response = self.client
            .post(&self.mailpace_endpoint)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("MailPace-Server-Token", token)
            .json(payload)
            .send()
            .await
            .context("Failed to send request to MailPace API")?;
        
        if response.status().is_success() {
            let mailpace_response: MailPaceResponse = response.json().await
                .context("Failed to parse MailPace response")?;
            info!("Email sent successfully, ID: {:?}", mailpace_response.id);
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            Err(anyhow::anyhow!("MailPace API error ({}): {}", status, error_text))
        }
    }
    
    async fn send_response(&self, connection: &mut Connection, response: &str) -> Result<()> {
        debug!("Sending response: {}", response);
        connection.write_all(format!("{}\r\n", response).as_bytes()).await?;
        connection.flush().await?;
        Ok(())
    }
    
    fn reset_session(&mut self) {
        self.mail_from = None;
        self.rcpt_to.clear();
        self.data.clear();
        self.state = SmtpState::Helo;
        // Don't reset auth_token - keep it for the session
    }
}

// Default certificates for testing (self-signed)
const DEFAULT_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIBkTCB+wIJAMlyFqk69v+9MA0GCSqGSIb3DQEBCwUAMBQxEjAQBgNVBAMMCWxv
Y2FsaG9zdDAeFw0yMzEwMDEwMDAwMDBaFw0yNDEwMDEwMDAwMDBaMBQxEjAQBgNV
BAMMCWxvY2FsaG9zdDBcMA0GCSqGSIb3DQEBAQUAA0sAMEgCQQDTgvwjlRHZ9M7+
OSKEbf2gPG1KOoGMjcZKzp5YNz1JkJC2pGnAjMN+5yZVpJj5CjAzFBmU0jJCQPLs
xzGPFpRlAgMBAAEwDQYJKoZIhvcNAQELBQADQQAzGRCvqhPMQyqCHJZBEpGm7A1i
MhJJPfJiCNL1qhPnRfhVdm7xzGGvxzLHjOBPgzJJSJgGDVjlHnNgvzADdBcq
-----END CERTIFICATE-----"#;

const DEFAULT_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
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
TANBgkqhkiG9w0BAQEFAAOCAQ==
-----END PRIVATE KEY-----"#;

fn load_tls_config() -> Result<Option<TlsAcceptor>> {
    let private_key_base64 = std::env::var("PRIVATEKEY");
    let cert_base64 = std::env::var("FULLCHAIN");
    
    let (private_key_pem, cert_pem) = match (private_key_base64, cert_base64) {
        (Ok(key_b64), Ok(cert_b64)) => {
            info!("Loading TLS certificates from environment variables");
            let key_pem = String::from_utf8(
                general_purpose::STANDARD.decode(key_b64)
                    .context("Failed to decode PRIVATEKEY base64")?
            ).context("PRIVATEKEY is not valid UTF-8")?;
            
            let cert_pem = String::from_utf8(
                general_purpose::STANDARD.decode(cert_b64)
                    .context("Failed to decode FULLCHAIN base64")?
            ).context("FULLCHAIN is not valid UTF-8")?;
            
            (key_pem, cert_pem)
        }
        _ => {
            warn!("TLS environment variables not found, using default test certificates");
            (DEFAULT_KEY_PEM.to_string(), DEFAULT_CERT_PEM.to_string())
        }
    };
    
    // Parse certificates
    let mut cert_reader = std::io::Cursor::new(cert_pem.as_bytes());
    let cert_chain = certs(&mut cert_reader)?
        .into_iter()
        .map(Certificate)
        .collect();
    
    // Parse private key
    let mut key_reader = std::io::Cursor::new(private_key_pem.as_bytes());
    let private_keys = pkcs8_private_keys(&mut key_reader)?;
    
    if private_keys.is_empty() {
        return Err(anyhow::anyhow!("No private keys found"));
    }
    
    let private_key = PrivateKey(private_keys[0].clone());
    
    // Create TLS config
    let tls_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .context("Failed to create TLS config")?;
    
    Ok(Some(TlsAcceptor::from(Arc::new(tls_config))))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let filter = if args.debug {
        "debug"
    } else {
        "info"
    };
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(filter))
        .init();
    
    let client = Client::new();
    let listener = TcpListener::bind(&args.listen).await
        .context("Failed to bind to address")?;
    
    info!("SMTP server listening on {}", args.listen);
    
    if args.default_mailpace_token.is_none() {
        info!("No default MailPace API token provided. Users must authenticate with their API token via SMTP AUTH.");
    } else {
        info!("Default MailPace API token loaded from environment. Users can override via SMTP AUTH.");
    }
    
    // Load TLS configuration if enabled
    let tls_acceptor = if args.enable_tls {
        match load_tls_config() {
            Ok(Some(acceptor)) => {
                info!("TLS configuration loaded");
                Some(acceptor)
            }
            Ok(None) => {
                warn!("TLS configuration not found, continuing without TLS");
                None
            }
            Err(e) => {
                error!("Error loading TLS configuration: {}", e);
                return Err(e);
            }
        }
    } else {
        None
    };
    
    while let Ok((stream, addr)) = listener.accept().await {
        info!("New connection from {}", addr);
        
        let client = client.clone();
        let mailpace_endpoint = args.mailpace_endpoint.clone();
        let default_mailpace_token = args.default_mailpace_token.clone();
        let tls_acceptor = tls_acceptor.clone();
        
        tokio::spawn(async move {
            let mut session = SmtpSession::new(client, mailpace_endpoint, default_mailpace_token, tls_acceptor);
            if let Err(e) = session.handle(stream).await {
                error!("Session error for {}: {}", addr, e);
            }
            info!("Connection closed for {}", addr);
        });
    }
    
    Ok(())
}
