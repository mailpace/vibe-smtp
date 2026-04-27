use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Client;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::time::Duration;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod compression;
mod connection;
mod mailpace;
mod mime;
mod smtp;
mod tls;

use cli::Args;
use mailpace::MailPaceClient;
use smtp::SmtpSession;

#[derive(Clone)]
struct ServerConfig {
    client: Client,
    mailpace_endpoint: String,
    mailpace_retries: usize,
    mailpace_retry_backoff: Duration,
    default_mailpace_token: Option<String>,
    enable_attachments: bool,
    max_attachment_size: usize,
    max_attachments: usize,
    enable_html_compression: bool,
    max_command_length: usize,
    max_message_size: usize,
    max_recipients: usize,
    read_timeout: Duration,
    write_timeout: Duration,
    max_session_duration: Duration,
    connection_limit: Arc<Semaphore>,
}

#[derive(Clone, Copy)]
enum TlsMode {
    None,
    Starttls,
    Implicit,
}

async fn start_listener(
    address: String,
    tls_mode: TlsMode,
    config: ServerConfig,
    tls_acceptor: Option<tokio_rustls::TlsAcceptor>,
) -> Result<()> {
    let listener = TcpListener::bind(&address)
        .await
        .context(format!("Failed to bind to {address}"))?;

    let tls_mode_str = match tls_mode {
        TlsMode::None => "Plain",
        TlsMode::Starttls => "STARTTLS",
        TlsMode::Implicit => "Implicit TLS",
    };

    info!("SMTP server listening on {} ({})", address, tls_mode_str);

    loop {
        let (mut stream, addr) = listener.accept().await?;
        info!("New connection from {} on {}", addr, address);

        let permit = match config.connection_limit.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Connection limit reached; rejecting {} on {}",
                    addr, address
                );
                use tokio::io::AsyncWriteExt;
                let _ = stream.write_all(b"421 Too many connections\r\n").await;
                let _ = stream.flush().await;
                continue;
            }
        };

        let config = config.clone();
        let tls_acceptor = tls_acceptor.clone();
        let address_clone = address.clone();

        tokio::spawn(async move {
            let _permit = permit;
            let result = match tls_mode {
                TlsMode::Implicit => {
                    // For implicit TLS (port 465), immediately upgrade to TLS
                    if let Some(acceptor) = tls_acceptor {
                        match acceptor.accept(stream).await {
                            Ok(tls_stream) => {
                                let mailpace_client =
                                    MailPaceClient::new(
                                        config.client,
                                        config.mailpace_endpoint,
                                        config.mailpace_retries,
                                        config.mailpace_retry_backoff,
                                    );
                                let mut session = SmtpSession::new(
                                    mailpace_client,
                                    config.default_mailpace_token,
                                    None, // No STARTTLS for implicit TLS
                                    config.enable_attachments,
                                    config.max_attachment_size,
                                    config.max_attachments,
                                    config.enable_html_compression,
                                    config.max_command_length,
                                    config.max_message_size,
                                    config.max_recipients,
                                    config.read_timeout,
                                    config.write_timeout,
                                    config.max_session_duration,
                                );
                                session.handle_tls_stream(Box::new(tls_stream)).await
                            }
                            Err(e) => {
                                error!(
                                    "Failed to establish implicit TLS connection for {}: {}",
                                    addr, e
                                );
                                return;
                            }
                        }
                    } else {
                        error!("Implicit TLS requested but no TLS acceptor configured");
                        return;
                    }
                }
                _ => {
                    // For plain and STARTTLS modes, start with plain connection
                    let session_tls_acceptor = match tls_mode {
                        TlsMode::Starttls => tls_acceptor,
                        _ => None,
                    };

                    let mailpace_client =
                        MailPaceClient::new(
                            config.client,
                            config.mailpace_endpoint,
                            config.mailpace_retries,
                            config.mailpace_retry_backoff,
                        );
                    let mut session = SmtpSession::new(
                        mailpace_client,
                        config.default_mailpace_token,
                        session_tls_acceptor,
                        config.enable_attachments,
                        config.max_attachment_size,
                        config.max_attachments,
                        config.enable_html_compression,
                        config.max_command_length,
                        config.max_message_size,
                        config.max_recipients,
                        config.read_timeout,
                        config.write_timeout,
                        config.max_session_duration,
                    );
                    session.handle(stream).await
                }
            };

            if let Err(e) = result {
                error!("Session error for {} on {}: {}", addr, address_clone, e);
            }
            info!("Connection closed for {} on {}", addr, address_clone);
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.debug { "debug" } else { "info" };

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(filter))
        .init();

    if args.default_mailpace_token.is_none() {
        info!("No default MailPace API token provided. Users must authenticate with their API token via SMTP AUTH.");
    } else {
        info!(
            "Default MailPace API token loaded from environment. Users can override via SMTP AUTH."
        );
    }

    // Log attachment configuration
    if args.enable_attachments {
        info!(
            "Attachment support enabled: max {} attachments, max size {} bytes each",
            args.max_attachments, args.max_attachment_size
        );
    } else {
        info!("Attachment support disabled");
    }

    // Log HTML compression configuration
    if args.enable_html_compression {
        info!("HTML compression enabled for email bodies");
    } else {
        info!("HTML compression disabled");
    }

    // Load TLS configuration if needed
    let tls_acceptor = if args.enable_tls || args.docker_multi_port {
        match tls::load_tls_config() {
            Ok(Some(acceptor)) => {
                info!("TLS configuration loaded");
                Some(acceptor)
            }
            Ok(None) => {
                if args.docker_multi_port {
                    error!("Docker multi-port mode requires TLS configuration, but none found");
                    return Err(anyhow::anyhow!(
                        "TLS configuration required for Docker multi-port mode"
                    ));
                }
                info!("TLS configuration not found, continuing without TLS");
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

    let request_timeout = Duration::from_secs(args.mailpace_timeout_secs);
    let client = Client::builder()
        .connect_timeout(request_timeout)
        .timeout(request_timeout)
        .build()
        .context("Failed to create MailPace HTTP client")?;

    let config = ServerConfig {
        client,
        mailpace_endpoint: args.mailpace_endpoint.clone(),
        mailpace_retries: args.mailpace_retries,
        mailpace_retry_backoff: Duration::from_millis(args.mailpace_retry_backoff_ms),
        default_mailpace_token: args.default_mailpace_token.clone(),
        enable_attachments: args.enable_attachments,
        max_attachment_size: args.max_attachment_size,
        max_attachments: args.max_attachments,
        enable_html_compression: args.enable_html_compression,
        max_command_length: args.max_command_length,
        max_message_size: args.max_message_size,
        max_recipients: args.max_recipients,
        read_timeout: Duration::from_secs(args.read_timeout_secs),
        write_timeout: Duration::from_secs(args.write_timeout_secs),
        max_session_duration: Duration::from_secs(args.max_session_duration_secs),
        connection_limit: Arc::new(Semaphore::new(args.max_connections)),
    };

    if args.docker_multi_port {
        info!("Starting Docker multi-port mode");

        // Start multiple listeners
        let mut handles = vec![];

        // Port 25 - Standard SMTP with STARTTLS
        handles.push(tokio::spawn(start_listener(
            "0.0.0.0:25".to_string(),
            TlsMode::Starttls,
            config.clone(),
            tls_acceptor.clone(),
        )));

        // Port 587 - Message Submission with STARTTLS
        handles.push(tokio::spawn(start_listener(
            "0.0.0.0:587".to_string(),
            TlsMode::Starttls,
            config.clone(),
            tls_acceptor.clone(),
        )));

        // Port 2525 - Alternative SMTP with STARTTLS
        handles.push(tokio::spawn(start_listener(
            "0.0.0.0:2525".to_string(),
            TlsMode::Starttls,
            config.clone(),
            tls_acceptor.clone(),
        )));

        // Port 465 - SMTP over SSL (implicit TLS)
        handles.push(tokio::spawn(start_listener(
            "0.0.0.0:465".to_string(),
            TlsMode::Implicit,
            config.clone(),
            tls_acceptor.clone(),
        )));

        // Wait for all listeners
        for handle in handles {
            if let Err(e) = handle.await? {
                error!("Listener error: {}", e);
                return Err(e);
            }
        }
    } else {
        // Single port mode (original behavior)
        info!("Starting single-port mode");

        let tls_mode = if args.enable_tls {
            TlsMode::Starttls
        } else {
            TlsMode::None
        };

        start_listener(args.listen.clone(), tls_mode, config, tls_acceptor).await?;
    }

    Ok(())
}
