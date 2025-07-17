use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

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

pub fn load_tls_config() -> Result<Option<TlsAcceptor>> {
    let private_key_base64 = std::env::var("PRIVATEKEY");
    let cert_base64 = std::env::var("FULLCHAIN");

    let (private_key_pem, cert_pem) = match (private_key_base64, cert_base64) {
        (Ok(key_b64), Ok(cert_b64)) => {
            info!("Loading TLS certificates from environment variables");
            let key_pem = String::from_utf8(
                general_purpose::STANDARD
                    .decode(key_b64)
                    .context("Failed to decode PRIVATEKEY base64")?,
            )
            .context("PRIVATEKEY is not valid UTF-8")?;

            let cert_pem = String::from_utf8(
                general_purpose::STANDARD
                    .decode(cert_b64)
                    .context("Failed to decode FULLCHAIN base64")?,
            )
            .context("FULLCHAIN is not valid UTF-8")?;

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
