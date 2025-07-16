use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// SMTP server listen address
    #[arg(short, long, default_value = "127.0.0.1:2525")]
    pub listen: String,
    
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
}
