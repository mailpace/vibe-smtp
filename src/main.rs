use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Client;
use tokio::net::TcpListener;
use tracing::{error, info};
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.debug { "debug" } else { "info" };

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(filter))
        .init();

    let client = Client::new();
    let listener = TcpListener::bind(&args.listen)
        .await
        .context("Failed to bind to address")?;

    info!("SMTP server listening on {}", args.listen);

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

    // Load TLS configuration if enabled
    let tls_acceptor = if args.enable_tls {
        match tls::load_tls_config() {
            Ok(Some(acceptor)) => {
                info!("TLS configuration loaded");
                Some(acceptor)
            }
            Ok(None) => {
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

    while let Ok((stream, addr)) = listener.accept().await {
        info!("New connection from {}", addr);

        let client = client.clone();
        let mailpace_endpoint = args.mailpace_endpoint.clone();
        let default_mailpace_token = args.default_mailpace_token.clone();
        let tls_acceptor = tls_acceptor.clone();
        let enable_attachments = args.enable_attachments;
        let max_attachment_size = args.max_attachment_size;
        let max_attachments = args.max_attachments;
        let enable_html_compression = args.enable_html_compression;

        tokio::spawn(async move {
            let mailpace_client = MailPaceClient::new(client, mailpace_endpoint);
            let mut session = SmtpSession::new(
                mailpace_client,
                default_mailpace_token,
                tls_acceptor,
                enable_attachments,
                max_attachment_size,
                max_attachments,
                enable_html_compression,
            );
            if let Err(e) = session.handle(stream).await {
                error!("Session error for {}: {}", addr, e);
            }
            info!("Connection closed for {}", addr);
        });
    }

    Ok(())
}
