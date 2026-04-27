use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};

use crate::compression::HtmlCompressor;
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
    html_compressor: Option<HtmlCompressor>,
}

impl SmtpSession {
    pub fn new(
        mailpace_client: MailPaceClient,
        default_mailpace_token: Option<String>,
        tls_acceptor: Option<TlsAcceptor>,
        enable_attachments: bool,
        max_attachment_size: usize,
        max_attachments: usize,
        enable_html_compression: bool,
    ) -> Self {
        let supports_starttls = tls_acceptor.is_some();
        let html_compressor = if enable_html_compression {
            Some(HtmlCompressor::new())
        } else {
            None
        };

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
            html_compressor,
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
                        connection = Connection::Tls(Box::new(tls_stream));
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

    pub async fn handle_tls_stream(
        &mut self,
        tls_stream: Box<tokio_rustls::server::TlsStream<TcpStream>>,
    ) -> Result<()> {
        let mut connection = Connection::Tls(tls_stream);
        self.send_response(&mut connection, "220 vibe-gateway SMTP ready")
            .await?;

        loop {
            let mut line = String::new();

            // Read command from TLS stream
            let command = match &mut connection {
                Connection::Tls(stream) => {
                    let mut reader = BufReader::new(stream);
                    let bytes_read = reader.read_line(&mut line).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    line.trim().to_string()
                }
                _ => {
                    return Err(anyhow::anyhow!("Expected TLS connection"));
                }
            };

            debug!("Received command: {}", command);

            // STARTTLS is not supported on implicit TLS connections
            if command.to_uppercase() == "STARTTLS" {
                self.send_response(&mut connection, "454 TLS already active")
                    .await?;
                continue;
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
                self.handle_data_processing(&mut connection).await?;
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
                self.send_response(connection, &format!("550 Error: {e}"))
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
                        let mut response =
                            vec!["250-vibe-gateway".to_string(), "250-AUTH PLAIN".to_string()];

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
                            // Only support single-line AUTH PLAIN with an initial response.
                            if parts.len() < 3 {
                                return Ok(Some(
                                    "504 Unsupported AUTH flow: use AUTH PLAIN with initial response"
                                        .to_string(),
                                ));
                            }

                            // AUTH PLAIN initial response - decode and extract token.
                            let decoded = match general_purpose::STANDARD.decode(parts[2]) {
                                Ok(decoded) => decoded,
                                Err(_) => {
                                    return Ok(Some(
                                        "535 Authentication credentials invalid".to_string(),
                                    ))
                                }
                            };

                            let auth_string = match String::from_utf8(decoded) {
                                Ok(auth_string) => auth_string,
                                Err(_) => {
                                    return Ok(Some(
                                        "535 Authentication credentials invalid".to_string(),
                                    ))
                                }
                            };

                            // PLAIN format: \0username\0password
                            // For MailPace, both username and password are expected to be the API token.
                            let auth_parts: Vec<&str> = auth_string.split('\0').collect();
                            if auth_parts.len() < 3
                                || auth_parts[1].is_empty()
                                || auth_parts[1] != auth_parts[2]
                            {
                                return Ok(Some(
                                    "535 Authentication credentials invalid".to_string(),
                                ));
                            }

                            self.auth_token = Some(auth_parts[1].to_string());
                            Ok(Some("235 Authentication successful".to_string()))
                        }
                        "LOGIN" => Ok(Some(
                            "504 Unsupported AUTH mechanism: LOGIN; use AUTH PLAIN".to_string(),
                        )),
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

        // Simple HTML detection and compression
        let (htmlbody, textbody) = if body.contains("<html>") || body.contains("<HTML>") {
            let compressed_html = if let Some(ref compressor) = self.html_compressor {
                match compressor.compress_html(&body) {
                    Ok(compressed) => compressed,
                    Err(e) => {
                        warn!("Failed to compress HTML: {}, using original", e);
                        body
                    }
                }
            } else {
                body
            };
            (Some(compressed_html), None)
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
            .write_all(format!("{response}\r\n").as_bytes())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mailpace::MailPaceClient;
    use reqwest::Client;
    use tokio::io::AsyncReadExt;
    use tokio::net::{TcpListener, TcpStream};
    use tokio_test::io::Builder;

    fn create_test_session() -> SmtpSession {
        let client = Client::new();
        let mailpace_client =
            MailPaceClient::new(client, "https://api.mailpace.com/v1/send".to_string());
        SmtpSession::new(
            mailpace_client,
            Some("test-token".to_string()),
            None,
            false,
            1024 * 1024, // 1MB
            5,
            false, // HTML compression disabled for most tests
        )
    }

    fn create_test_session_with_attachments() -> SmtpSession {
        let client = Client::new();
        let mailpace_client =
            MailPaceClient::new(client, "https://api.mailpace.com/v1/send".to_string());
        SmtpSession::new(
            mailpace_client,
            Some("test-token".to_string()),
            None,
            true,
            1024 * 1024, // 1MB
            5,
            false, // HTML compression disabled for most tests
        )
    }

    fn create_test_session_with_html_compression() -> SmtpSession {
        let client = Client::new();
        let mailpace_client =
            MailPaceClient::new(client, "https://api.mailpace.com/v1/send".to_string());
        SmtpSession::new(
            mailpace_client,
            Some("test-token".to_string()),
            None,
            false,
            1024 * 1024, // 1MB
            5,
            true, // HTML compression enabled
        )
    }

    #[test]
    fn test_smtp_state_equality() {
        assert_eq!(SmtpState::Init, SmtpState::Init);
        assert_eq!(SmtpState::Helo, SmtpState::Helo);
        assert_eq!(SmtpState::Mail, SmtpState::Mail);
        assert_eq!(SmtpState::Rcpt, SmtpState::Rcpt);
        assert_eq!(SmtpState::Data, SmtpState::Data);
        assert_eq!(SmtpState::Quit, SmtpState::Quit);

        assert_ne!(SmtpState::Init, SmtpState::Helo);
    }

    #[test]
    fn test_smtp_session_new() {
        let session = create_test_session();
        assert_eq!(session.state, SmtpState::Init);
        assert_eq!(session.helo, None);
        assert_eq!(session.mail_from, None);
        assert!(session.rcpt_to.is_empty());
        assert!(session.data.is_empty());
        assert_eq!(session.auth_token, None);
        assert!(!session.supports_starttls);
        assert!(!session.enable_attachments);
        assert_eq!(session.max_attachment_size, 1024 * 1024);
        assert_eq!(session.max_attachments, 5);
    }

    #[test]
    fn test_smtp_session_new_with_attachments() {
        let session = create_test_session_with_attachments();
        assert!(session.enable_attachments);
    }

    #[test]
    fn test_smtp_session_new_with_html_compression() {
        let session = create_test_session_with_html_compression();
        assert!(session.html_compressor.is_some());

        let session_no_compression = create_test_session();
        assert!(session_no_compression.html_compressor.is_none());
    }

    #[tokio::test]
    async fn test_process_command_helo() {
        let mut session = create_test_session();

        let result = session.process_command("HELO example.com").await.unwrap();
        assert_eq!(result, Some("250 vibe-gateway".to_string()));
        assert_eq!(session.state, SmtpState::Helo);
        assert_eq!(session.helo, Some("example.com".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_ehlo() {
        let mut session = create_test_session();

        let result = session.process_command("EHLO example.com").await.unwrap();
        assert!(result.is_some());
        let response = result.unwrap();
        assert!(response.contains("250-vibe-gateway"));
        assert!(response.contains("250-AUTH PLAIN"));
        assert!(response.contains("250 8BITMIME"));
        assert_eq!(session.state, SmtpState::Helo);
        assert_eq!(session.helo, Some("example.com".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_ehlo_with_attachments() {
        let mut session = create_test_session_with_attachments();

        let result = session.process_command("EHLO example.com").await.unwrap();
        assert!(result.is_some());
        let response = result.unwrap();
        assert!(response.contains("250-ENHANCEDSTATUSCODES"));
        assert!(response.contains("250-SIZE"));
    }

    #[tokio::test]
    async fn test_process_command_helo_no_domain() {
        let mut session = create_test_session();

        let result = session.process_command("HELO").await.unwrap();
        assert_eq!(result, Some("501 Syntax error in parameters".to_string()));
        assert_eq!(session.state, SmtpState::Init);
    }

    #[tokio::test]
    async fn test_process_command_auth_plain() {
        let mut session = create_test_session();

        // Test AUTH PLAIN with base64 encoded credentials
        let auth_string = "\0test-token\0test-token";
        let encoded = base64::engine::general_purpose::STANDARD.encode(auth_string);
        let command = format!("AUTH PLAIN {encoded}");

        let result = session.process_command(&command).await.unwrap();
        assert_eq!(result, Some("235 Authentication successful".to_string()));
        assert_eq!(session.auth_token, Some("test-token".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_auth_plain_multipart() {
        let mut session = create_test_session();

        let result = session.process_command("AUTH PLAIN").await.unwrap();
        assert_eq!(
            result,
            Some("504 Unsupported AUTH flow: use AUTH PLAIN with initial response".to_string())
        );
    }

    #[tokio::test]
    async fn test_process_command_auth_login() {
        let mut session = create_test_session();

        let result = session.process_command("AUTH LOGIN").await.unwrap();
        assert_eq!(
            result,
            Some("504 Unsupported AUTH mechanism: LOGIN; use AUTH PLAIN".to_string())
        );
    }

    #[tokio::test]
    async fn test_process_command_auth_plain_invalid_base64() {
        let mut session = create_test_session();

        let result = session
            .process_command("AUTH PLAIN not-valid-base64")
            .await
            .unwrap();
        assert_eq!(
            result,
            Some("535 Authentication credentials invalid".to_string())
        );
    }

    #[tokio::test]
    async fn test_process_command_auth_plain_mismatched_username_password() {
        let mut session = create_test_session();

        let auth_string = "\0testuser\0different";
        let encoded = base64::engine::general_purpose::STANDARD.encode(auth_string);
        let command = format!("AUTH PLAIN {encoded}");
        let result = session.process_command(&command).await.unwrap();

        assert_eq!(
            result,
            Some("535 Authentication credentials invalid".to_string())
        );
    }

    #[tokio::test]
    async fn test_process_command_auth_unsupported() {
        let mut session = create_test_session();

        let result = session.process_command("AUTH DIGEST-MD5").await.unwrap();
        assert_eq!(
            result,
            Some("504 Unrecognized authentication type".to_string())
        );
    }

    #[tokio::test]
    async fn test_process_command_auth_no_params() {
        let mut session = create_test_session();

        let result = session.process_command("AUTH").await.unwrap();
        assert_eq!(result, Some("501 Syntax error in parameters".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_mail_from() {
        let mut session = create_test_session();

        let result = session
            .process_command("MAIL FROM:<test@example.com>")
            .await
            .unwrap();
        assert_eq!(result, Some("250 OK".to_string()));
        assert_eq!(session.state, SmtpState::Mail);
        assert_eq!(session.mail_from, Some("test@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_mail_from_no_brackets() {
        let mut session = create_test_session();

        let result = session
            .process_command("MAIL FROM: test@example.com")
            .await
            .unwrap();
        assert_eq!(result, Some("250 OK".to_string()));
        assert_eq!(session.mail_from, Some("test@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_mail_invalid() {
        let mut session = create_test_session();

        let result = session
            .process_command("MAIL TO:test@example.com")
            .await
            .unwrap();
        assert_eq!(result, Some("501 Syntax error in parameters".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_rcpt_to() {
        let mut session = create_test_session();

        let result = session
            .process_command("RCPT TO:<recipient@example.com>")
            .await
            .unwrap();
        assert_eq!(result, Some("250 OK".to_string()));
        assert_eq!(session.state, SmtpState::Rcpt);
        assert_eq!(session.rcpt_to, vec!["recipient@example.com"]);
    }

    #[tokio::test]
    async fn test_process_command_rcpt_to_multiple() {
        let mut session = create_test_session();

        let _ = session
            .process_command("RCPT TO:<recipient1@example.com>")
            .await
            .unwrap();
        let result = session
            .process_command("RCPT TO:<recipient2@example.com>")
            .await
            .unwrap();

        assert_eq!(result, Some("250 OK".to_string()));
        assert_eq!(
            session.rcpt_to,
            vec!["recipient1@example.com", "recipient2@example.com"]
        );
    }

    #[tokio::test]
    async fn test_process_command_rcpt_invalid() {
        let mut session = create_test_session();

        let result = session
            .process_command("RCPT FROM:test@example.com")
            .await
            .unwrap();
        assert_eq!(result, Some("501 Syntax error in parameters".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_data_success() {
        let mut session = create_test_session();
        session.mail_from = Some("sender@example.com".to_string());
        session.rcpt_to.push("recipient@example.com".to_string());

        let result = session.process_command("DATA").await.unwrap();
        assert_eq!(
            result,
            Some("354 End data with <CR><LF>.<CR><LF>".to_string())
        );
        assert_eq!(session.state, SmtpState::Data);
    }

    #[tokio::test]
    async fn test_process_command_data_no_sender() {
        let mut session = create_test_session();
        session.rcpt_to.push("recipient@example.com".to_string());

        let result = session.process_command("DATA").await.unwrap();
        assert_eq!(result, Some("503 Bad sequence of commands".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_data_no_recipients() {
        let mut session = create_test_session();
        session.mail_from = Some("sender@example.com".to_string());

        let result = session.process_command("DATA").await.unwrap();
        assert_eq!(result, Some("503 Bad sequence of commands".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_rset() {
        let mut session = create_test_session();
        session.mail_from = Some("sender@example.com".to_string());
        session.rcpt_to.push("recipient@example.com".to_string());
        session.data = b"test data".to_vec();
        session.state = SmtpState::Data;

        let result = session.process_command("RSET").await.unwrap();
        assert_eq!(result, Some("250 OK".to_string()));
        assert_eq!(session.state, SmtpState::Helo);
        assert_eq!(session.mail_from, None);
        assert!(session.rcpt_to.is_empty());
        assert!(session.data.is_empty());
    }

    #[tokio::test]
    async fn test_process_command_noop() {
        let mut session = create_test_session();

        let result = session.process_command("NOOP").await.unwrap();
        assert_eq!(result, Some("250 OK".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_quit() {
        let mut session = create_test_session();

        let result = session.process_command("QUIT").await.unwrap();
        assert_eq!(result, Some("221 Goodbye".to_string()));
        assert_eq!(session.state, SmtpState::Quit);
    }

    #[tokio::test]
    async fn test_process_command_unknown() {
        let mut session = create_test_session();

        let result = session.process_command("UNKNOWN").await.unwrap();
        assert_eq!(result, Some("502 Command not implemented".to_string()));
    }

    #[tokio::test]
    async fn test_process_command_empty() {
        let mut session = create_test_session();

        let result = session.process_command("").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_read_data() {
        let mut session = create_test_session();

        let data = "Subject: Test\r\n\r\nThis is the body\r\n.\r\n";
        let mock = Builder::new().read(data.as_bytes()).build();
        let mut reader = BufReader::new(mock);

        let result = session.read_data(&mut reader).await;
        assert!(result.is_ok());

        let data_str = String::from_utf8(session.data.clone()).unwrap();
        assert!(data_str.contains("Subject: Test"));
        assert!(data_str.contains("This is the body"));
        assert!(!data_str.contains(".\r\n"));
    }

    #[tokio::test]
    async fn test_read_data_with_dot_stuffing() {
        let mut session = create_test_session();

        let data = "Subject: Test\r\n\r\n..This line starts with two dots\r\n.\r\n";
        let mock = Builder::new().read(data.as_bytes()).build();
        let mut reader = BufReader::new(mock);

        let result = session.read_data(&mut reader).await;
        assert!(result.is_ok());

        let data_str = String::from_utf8(session.data.clone()).unwrap();
        assert!(data_str.contains(".This line starts with two dots"));
    }

    #[test]
    fn test_parse_email_to_mailpace_payload_simple() {
        let mut session = create_test_session();
        session.mail_from = Some("sender@example.com".to_string());
        session.rcpt_to = vec!["recipient@example.com".to_string()];

        let email_content =
            "Subject: Test Subject\r\nFrom: sender@example.com\r\n\r\nTest body content";

        let result = session.parse_email_to_mailpace_payload(email_content);
        assert!(result.is_ok());

        let payload = result.unwrap();
        assert_eq!(payload.from, "sender@example.com");
        assert_eq!(payload.to, "recipient@example.com");
        assert_eq!(payload.subject, Some("Test Subject".to_string()));
        assert_eq!(payload.textbody, Some("Test body content".to_string()));
        assert_eq!(payload.htmlbody, None);
    }

    #[test]
    fn test_parse_email_to_mailpace_payload_html() {
        let mut session = create_test_session();
        session.mail_from = Some("sender@example.com".to_string());
        session.rcpt_to = vec!["recipient@example.com".to_string()];

        let email_content =
            "Subject: Test Subject\r\n\r\n<html><body>Test HTML content</body></html>";

        let result = session.parse_email_to_mailpace_payload(email_content);
        assert!(result.is_ok());

        let payload = result.unwrap();
        assert_eq!(
            payload.htmlbody,
            Some("<html><body>Test HTML content</body></html>".to_string())
        );
        assert_eq!(payload.textbody, None);
    }

    #[test]
    fn test_parse_email_to_mailpace_payload_html_with_compression() {
        let mut session = create_test_session_with_html_compression();
        session.mail_from = Some("sender@example.com".to_string());
        session.rcpt_to = vec!["recipient@example.com".to_string()];

        let email_content = r#"Subject: Test Subject

<html>
    <head>
        <title>Test Email</title>
        <!-- This comment should be removed -->
    </head>
    <body>
        <h1>   Hello World   </h1>
        <p>This is a test email with lots of     whitespace.</p>
        <!-- Another comment -->
    </body>
</html>"#;

        let result = session.parse_email_to_mailpace_payload(email_content);
        assert!(result.is_ok());

        let payload = result.unwrap();
        let html_body = payload.htmlbody.unwrap();

        // Compressed HTML should be smaller than original
        let original_html = email_content.lines().skip(2).collect::<Vec<_>>().join("\n");
        assert!(html_body.len() < original_html.len());

        // Should still contain the essential content
        assert!(html_body.contains("Hello World"));
        assert!(html_body.contains("This is a test email"));

        // Comments should be removed
        assert!(!html_body.contains("This comment should be removed"));
        assert!(!html_body.contains("Another comment"));

        assert_eq!(payload.textbody, None);
    }

    #[test]
    fn test_parse_email_to_mailpace_payload_multiple_recipients() {
        let mut session = create_test_session();
        session.mail_from = Some("sender@example.com".to_string());
        session.rcpt_to = vec![
            "recipient1@example.com".to_string(),
            "recipient2@example.com".to_string(),
        ];

        let email_content = "Subject: Test Subject\r\n\r\nTest body";

        let result = session.parse_email_to_mailpace_payload(email_content);
        assert!(result.is_ok());

        let payload = result.unwrap();
        assert_eq!(payload.to, "recipient1@example.com, recipient2@example.com");
    }

    #[test]
    fn test_parse_email_to_mailpace_payload_with_headers() {
        let mut session = create_test_session();
        session.mail_from = Some("sender@example.com".to_string());
        session.rcpt_to = vec!["recipient@example.com".to_string()];

        let email_content = "Subject: Test Subject\r\nCc: cc@example.com\r\nBcc: bcc@example.com\r\nReply-To: reply@example.com\r\nX-MailPace-Tags: tag1,tag2\r\n\r\nTest body";

        let result = session.parse_email_to_mailpace_payload(email_content);
        assert!(result.is_ok());

        let payload = result.unwrap();
        assert_eq!(payload.subject, Some("Test Subject".to_string()));
        assert_eq!(payload.cc, Some("cc@example.com".to_string()));
        assert_eq!(payload.bcc, Some("bcc@example.com".to_string()));
        assert_eq!(payload.replyto, Some("reply@example.com".to_string()));
        assert_eq!(
            payload.tags,
            Some(vec!["tag1".to_string(), "tag2".to_string()])
        );
    }

    #[test]
    fn test_parse_email_to_mailpace_payload_no_sender() {
        let mut session = create_test_session();
        session.rcpt_to = vec!["recipient@example.com".to_string()];

        let email_content = "Subject: Test Subject\r\n\r\nTest body";

        let result = session.parse_email_to_mailpace_payload(email_content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No sender address"));
    }

    #[test]
    fn test_reset_session() {
        let mut session = create_test_session();
        session.mail_from = Some("sender@example.com".to_string());
        session.rcpt_to.push("recipient@example.com".to_string());
        session.data = b"test data".to_vec();
        session.state = SmtpState::Data;
        session.auth_token = Some("token".to_string());

        session.reset_session();

        assert_eq!(session.mail_from, None);
        assert!(session.rcpt_to.is_empty());
        assert!(session.data.is_empty());
        assert_eq!(session.state, SmtpState::Helo);
        // Auth token should be preserved
        assert_eq!(session.auth_token, Some("token".to_string()));
    }

    #[tokio::test]
    async fn test_send_response() {
        // Create a mock TcpStream for testing
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let stream = TcpStream::connect(addr).await.unwrap();
            let mut connection = Connection::Plain(stream);

            let session = create_test_session();
            let result = session.send_response(&mut connection, "250 OK").await;
            assert!(result.is_ok());
        });

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            let response = String::from_utf8_lossy(&buf[..n]);
            assert_eq!(response, "250 OK\r\n");
        });

        let _ = tokio::join!(client_task, server_task);
    }

    // Mock tests for the async handle method would require more complex setup
    // with mock TCP streams and would be better suited for integration tests
}
