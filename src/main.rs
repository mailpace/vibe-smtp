use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
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
    
    /// MailPace API token
    #[arg(long, env = "MAILPACE_API_TOKEN")]
    mailpace_token: Option<String>,
    
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
}

struct SmtpSession {
    client: Client,
    mailpace_endpoint: String,
    mailpace_token: Option<String>,
    state: SmtpState,
    helo: Option<String>,
    mail_from: Option<String>,
    rcpt_to: Vec<String>,
    data: Vec<u8>,
}

impl SmtpSession {
    fn new(client: Client, mailpace_endpoint: String, mailpace_token: Option<String>) -> Self {
        Self {
            client,
            mailpace_endpoint,
            mailpace_token,
            state: SmtpState::Init,
            helo: None,
            mail_from: None,
            rcpt_to: Vec::new(),
            data: Vec::new(),
        }
    }

    async fn handle(&mut self, mut stream: TcpStream) -> Result<()> {
        self.send_response(&mut stream, "220 vibe-gateway SMTP ready").await?;
        
        let mut line = String::new();
        
        loop {
            line.clear();
            
            // Read command
            {
                let mut reader = BufReader::new(&mut stream);
                let bytes_read = reader.read_line(&mut line).await?;
                if bytes_read == 0 {
                    break;
                }
            }
            
            let command = line.trim();
            debug!("Received command: {}", command);
            
            // Process command
            match self.process_command(command).await {
                Ok(response) => {
                    if let Some(resp) = response {
                        self.send_response(&mut stream, &resp).await?;
                    }
                }
                Err(e) => {
                    error!("Error processing command '{}': {}", command, e);
                    self.send_response(&mut stream, "451 Temporary local problem").await?;
                }
            }
            
            if self.state == SmtpState::Quit {
                break;
            }
            
            // Handle DATA command specially
            if self.state == SmtpState::Data {
                let mut reader = BufReader::new(&mut stream);
                if let Err(e) = self.read_data(&mut reader).await {
                    error!("Error reading data: {}", e);
                    self.send_response(&mut stream, "451 Error reading data").await?;
                } else {
                    match self.process_email_data().await {
                        Ok(_) => {
                            self.send_response(&mut stream, "250 OK: Message accepted for delivery").await?;
                        }
                        Err(e) => {
                            error!("Failed to send email to MailPace: {}", e);
                            self.send_response(&mut stream, &format!("550 Error: {}", e)).await?;
                        }
                    }
                    self.reset_session();
                }
            }
        }
        
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
                        Ok(Some("250-vibe-gateway\r\n250-AUTH PLAIN LOGIN\r\n250-STARTTLS\r\n250 8BITMIME".to_string()))
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
                        "PLAIN" | "LOGIN" => {
                            Ok(Some("235 Authentication successful".to_string()))
                        }
                        _ => {
                            Ok(Some("504 Unrecognized authentication type".to_string()))
                        }
                    }
                } else {
                    Ok(Some("501 Syntax error in parameters".to_string()))
                }
            }
            "STARTTLS" => {
                Ok(Some("220 Ready to start TLS".to_string()))
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
    
    async fn read_data(&mut self, reader: &mut BufReader<&mut TcpStream>) -> Result<()> {
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
        let token = self.mailpace_token.as_ref()
            .context("No MailPace API token provided")?;
        
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
    
    async fn send_response(&self, stream: &mut TcpStream, response: &str) -> Result<()> {
        debug!("Sending response: {}", response);
        stream.write_all(format!("{}\r\n", response).as_bytes()).await?;
        stream.flush().await?;
        Ok(())
    }
    
    fn reset_session(&mut self) {
        self.mail_from = None;
        self.rcpt_to.clear();
        self.data.clear();
        self.state = SmtpState::Helo;
    }
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
    
    if args.mailpace_token.is_none() {
        warn!("No MailPace API token provided. Set MAILPACE_API_TOKEN environment variable or use --mailpace-token");
    }
    
    while let Ok((stream, addr)) = listener.accept().await {
        info!("New connection from {}", addr);
        
        let client = client.clone();
        let mailpace_endpoint = args.mailpace_endpoint.clone();
        let mailpace_token = args.mailpace_token.clone();
        
        tokio::spawn(async move {
            let mut session = SmtpSession::new(client, mailpace_endpoint, mailpace_token);
            if let Err(e) = session.handle(stream).await {
                error!("Session error for {}: {}", addr, e);
            }
            info!("Connection closed for {}", addr);
        });
    }
    
    Ok(())
}
