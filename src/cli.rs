use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// SMTP server listen address (single port mode)
    #[arg(short, long, default_value = "127.0.0.1:2525")]
    pub listen: String,

    /// Enable multi-port Docker mode (ports 25, 587, 2525 with STARTTLS, 465 with implicit TLS)
    #[arg(long)]
    pub docker_multi_port: bool,

    /// MailPace API endpoint
    #[arg(long, default_value = "https://app.mailpace.com/api/v1/send")]
    pub mailpace_endpoint: String,

    /// Default MailPace API token (optional, can be overridden by SMTP auth)
    #[arg(long, env = "MAILPACE_API_TOKEN")]
    pub default_mailpace_token: Option<String>,

    /// Enable TLS/STARTTLS support
    #[arg(long)]
    pub enable_tls: bool,

    /// Debug mode
    #[arg(short, long)]
    pub debug: bool,

    /// Enable attachment support
    #[arg(long)]
    pub enable_attachments: bool,

    /// Maximum attachment size in bytes (default: 10MB)
    #[arg(long, default_value = "10485760")]
    pub max_attachment_size: usize,

    /// Maximum number of attachments per email (default: 10)
    #[arg(long, default_value = "10")]
    pub max_attachments: usize,

    /// Enable HTML compression for email bodies
    #[arg(long)]
    pub enable_html_compression: bool,

    /// Maximum concurrent SMTP sessions
    #[arg(long, default_value = "1000")]
    pub max_connections: usize,

    /// Maximum SMTP command length in bytes
    #[arg(long, default_value = "2048")]
    pub max_command_length: usize,

    /// Maximum SMTP message size in bytes
    #[arg(long, default_value = "26214400")]
    pub max_message_size: usize,

    /// Maximum recipients accepted per message
    #[arg(long, default_value = "100")]
    pub max_recipients: usize,

    /// Per-command socket read timeout in seconds
    #[arg(long, default_value = "30")]
    pub read_timeout_secs: u64,

    /// Socket write timeout in seconds
    #[arg(long, default_value = "30")]
    pub write_timeout_secs: u64,

    /// Maximum SMTP session duration in seconds
    #[arg(long, default_value = "300")]
    pub max_session_duration_secs: u64,

    /// MailPace API request timeout in seconds
    #[arg(long, default_value = "15")]
    pub mailpace_timeout_secs: u64,

    /// Number of retries for transient MailPace API failures
    #[arg(long, default_value = "2")]
    pub mailpace_retries: usize,

    /// Initial MailPace retry backoff in milliseconds
    #[arg(long, default_value = "250")]
    pub mailpace_retry_backoff_ms: u64,
}
