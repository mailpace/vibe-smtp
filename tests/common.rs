use anyhow::Result;
use lettre::transport::smtp::{authentication::Credentials, client::Tls, SmtpTransport};
use serde_json::json;
use std::{net::SocketAddr, time::Duration};
use tokio::{
    net::TcpListener,
    process::{Child, Command},
    time::sleep,
};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Mock MailPace API server for testing
pub struct MockMailPaceServer {
    pub server: MockServer,
}

impl MockMailPaceServer {
    pub async fn new() -> Self {
        let server = MockServer::start().await;

        Self { server }
    }

    pub async fn setup_success_response(&self) -> &Self {
        Mock::given(method("POST"))
            .and(path("/api/v1/send"))
            .and(header("Content-Type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "test-message-id",
                "status": "sent"
            })))
            .mount(&self.server)
            .await;

        self
    }

    pub async fn setup_error_response(&self, status: u16, message: &str) -> &Self {
        Mock::given(method("POST"))
            .and(path("/api/v1/send"))
            .respond_with(ResponseTemplate::new(status).set_body_json(json!({
                "errors": [message]
            })))
            .mount(&self.server)
            .await;

        self
    }
}

/// Test server manager
pub struct TestServer {
    pub child: Child,
    pub smtp_port: u16,
    pub mock_server: MockMailPaceServer,
}

impl TestServer {
    pub async fn new() -> Result<Self> {
        Self::new_with_config(&[]).await
    }

    pub async fn new_with_html_compression() -> Result<Self> {
        Self::new_with_config(&["--enable-html-compression"]).await
    }

    pub async fn new_with_attachments() -> Result<Self> {
        Self::new_with_config(&["--enable-attachments"]).await
    }

    pub async fn new_with_config(extra_args: &[&str]) -> Result<Self> {
        let mock_server = MockMailPaceServer::new().await;

        // Find available port for SMTP
        let smtp_listener = TcpListener::bind("127.0.0.1:0").await?;
        let smtp_port = smtp_listener.local_addr()?.port();
        drop(smtp_listener);

        // Create the formatted strings first to ensure they live long enough
        let listen_addr = format!("127.0.0.1:{smtp_port}");
        let mailpace_endpoint = format!("{}/api/v1/send", mock_server.server.uri());

        let mut base_args = vec![
            "--listen",
            &listen_addr,
            "--mailpace-endpoint",
            &mailpace_endpoint,
            "--debug",
        ];

        // Add extra configuration arguments
        for arg in extra_args {
            base_args.push(arg);
        }

        // Start the vibe-gateway server using pre-built binary to avoid cargo lock issues
        let child = Command::new("./target/release/vibe-gateway")
            .args(&base_args)
            .env("MAILPACE_API_TOKEN", "test-token")
            .spawn()
            .or_else(|_| {
                // Fallback to cargo run if binary doesn't exist
                let mut cargo_args = vec!["run", "--release", "--"];
                cargo_args.extend(&base_args);

                Command::new("cargo")
                    .args(&cargo_args)
                    .env("MAILPACE_API_TOKEN", "test-token")
                    .spawn()
            })?;

        let server = Self {
            child,
            smtp_port,
            mock_server,
        };

        // Wait for server to start
        server.wait_for_server().await?;

        Ok(server)
    }

    async fn wait_for_server(&self) -> Result<()> {
        for _ in 0..30 {
            if (tokio::net::TcpStream::connect(format!("127.0.0.1:{}", self.smtp_port)).await)
                .is_ok()
            {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }
        Err(anyhow::anyhow!("Server failed to start"))
    }

    pub fn smtp_address(&self) -> SocketAddr {
        format!("127.0.0.1:{}", self.smtp_port).parse().unwrap()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Kill the child process synchronously
        if let Err(e) = self.child.start_kill() {
            eprintln!("Failed to kill child process: {e}");
        }
        // Give the process time to clean up
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Helper function to create SMTP transport
pub fn create_smtp_transport(
    server_addr: SocketAddr,
    credentials: Option<Credentials>,
) -> SmtpTransport {
    let mut builder = SmtpTransport::builder_dangerous(server_addr.ip().to_string())
        .port(server_addr.port())
        .tls(Tls::None);

    if let Some(creds) = credentials {
        builder = builder.credentials(creds);
    }

    builder.build()
}
