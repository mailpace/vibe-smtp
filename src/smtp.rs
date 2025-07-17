use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info};

use crate::connection::Connection;
use crate::mailpace::{MailPaceClient, MailPacePayload};
use crate::mime::MimeParser;

#[derive(Debug, PartialEq)]
pub enum SmtpState {
    Init,
    Helo,
    Mail,
    Rcpt,
    Data,
    Quit,
    StartTls,
}

pub struct SmtpSession {
    mailpace_client: MailPaceClient,
    default_mailpace_token: Option<String>,
    state: SmtpState,
    helo: Option<String>,
    mail_from: Option<String>,
    rcpt_to: Vec<String>,
    data: Vec<u8>,
    auth_token: Option<String>,
    tls_acceptor: Option<TlsAcceptor>,
    supports_starttls: bool,
    enable_attachments: bool,
    max_attachment_size: usize,
    max_attachments: usize,
}

impl SmtpSession {
    pub fn new(
        mailpace_client: MailPaceClient,
        default_mailpace_token: Option<String>,
        tls_acceptor: Option<TlsAcceptor>,
        enable_attachments: bool,
        max_attachment_size: usize,
        max_attachments: usize,
    ) -> Self {
        let supports_starttls = tls_acceptor.is_some();
        Self {
            mailpace_client,
            default_mailpace_token,
            state: SmtpState::Init,
            helo: None,
            mail_from: None,
            rcpt_to: Vec::new(),
            data: Vec::new(),
            auth_token: None,
            tls_acceptor,
            supports_starttls,
            enable_attachments,
            max_attachment_size,
            max_attachments,
        }
    }

    pub async fn handle(&mut self, stream: TcpStream) -> Result<()> {
        let mut connection = Connection::Plain(stream);
        self.send_response(&mut connection, "220 vibe-gateway SMTP ready")
            .await?;

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
                    self.send_response(&mut connection, "220 Ready to start TLS")
                        .await?;

                    // Upgrade connection to TLS
                    if let Connection::Plain(plain_stream) = connection {
                        let tls_stream = acceptor
                            .accept(plain_stream)
                            .await
                            .context("Failed to establish TLS connection")?;
                        connection = Connection::Tls(tls_stream);
                        info!("TLS connection established");
                    }
                    continue;
                } else {
                    self.send_response(&mut connection, "454 TLS not available")
                        .await?;
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
                    self.send_response(&mut connection, "451 Temporary local problem")
                        .await?;
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
                            self.send_response(&mut connection, "451 Error reading data")
                                .await?;
                        } else {
                            self.handle_data_processing(&mut connection).await?;
                        }
                    }
                    Connection::Tls(stream) => {
                        let mut reader = BufReader::new(stream);
                        if let Err(e) = self.read_data(&mut reader).await {
                            error!("Error reading data: {}", e);
                            self.send_response(&mut connection, "451 Error reading data")
                                .await?;
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
                self.send_response(connection, "250 OK: Message accepted for delivery")
                    .await?;
            }
            Err(e) => {
                error!("Failed to send email to MailPace: {}", e);
                self.send_response(connection, &format!("550 Error: {}", e))
                    .await?;
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

                        if self.enable_attachments {
                            response.push("250-ENHANCEDSTATUSCODES".to_string());
                            response.push(format!(
                                "250-SIZE {}",
                                self.max_attachment_size * self.max_attachments
                            ));
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
                                            self.auth_token = Some(parts[1].to_string());
                                            // Use username as token
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
                        _ => Ok(Some("504 Unrecognized authentication type".to_string())),
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
            "NOOP" => Ok(Some("250 OK".to_string())),
            "QUIT" => {
                self.state = SmtpState::Quit;
                Ok(Some("221 Goodbye".to_string()))
            }
            _ => Ok(Some("502 Command not implemented".to_string())),
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
        let token = self
            .auth_token
            .as_ref()
            .or(self.default_mailpace_token.as_ref())
            .context("No MailPace API token provided via SMTP AUTH or default configuration")?;

        self.mailpace_client.send_email(&payload, token).await?;

        Ok(())
    }

    fn parse_email_to_mailpace_payload(&self, email_content: &str) -> Result<MailPacePayload> {
        let (headers, body, attachments) = if self.enable_attachments {
            let mime_parser = MimeParser::new(self.max_attachment_size, self.max_attachments);
            mime_parser.parse_email(email_content)?
        } else {
            // Simple parsing without attachment support
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

            (headers, body_lines.join("\n"), Vec::new())
        };

        let from = self
            .mail_from
            .as_ref()
            .context("No sender address")?
            .clone();

        let to = self.rcpt_to.join(", ");

        let subject = headers.get("subject").cloned();
        let cc = headers.get("cc").cloned();
        let bcc = headers.get("bcc").cloned();
        let replyto = headers.get("reply-to").cloned();
        let list_unsubscribe = headers
            .get("x-list-unsubscribe")
            .or_else(|| headers.get("list-unsubscribe"))
            .cloned();

        let tags = headers.get("x-mailpace-tags").map(|tags_str| {
            tags_str
                .split(',')
                .map(|tag| tag.trim().to_string())
                .collect::<Vec<_>>()
        });

        // Simple HTML detection
        let (htmlbody, textbody) = if body.contains("<html>") || body.contains("<HTML>") {
            (Some(body), None)
        } else {
            (None, Some(body))
        };

        let attachments = if attachments.is_empty() {
            None
        } else {
            Some(attachments)
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
            attachments,
            tags,
        })
    }

    async fn send_response(&self, connection: &mut Connection, response: &str) -> Result<()> {
        debug!("Sending response: {}", response);
        connection
            .write_all(format!("{}\r\n", response).as_bytes())
            .await?;
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
