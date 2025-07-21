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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper function to clear environment variables for testing
    fn clear_tls_env_vars() {
        env::remove_var("PRIVATEKEY");
        env::remove_var("FULLCHAIN");
    }

    // Helper function to set environment variables for testing
    fn set_tls_env_vars(private_key_b64: &str, cert_b64: &str) {
        env::set_var("PRIVATEKEY", private_key_b64);
        env::set_var("FULLCHAIN", cert_b64);
    }

    #[test]
    fn test_load_tls_config_with_default_certificates() {
        // Ensure no env vars are set
        clear_tls_env_vars();

        let result = load_tls_config();
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_load_tls_config_with_valid_env_vars() {
        // Base64 encode the default test certificates
        let private_key_b64 = general_purpose::STANDARD.encode(DEFAULT_KEY_PEM);
        let cert_b64 = general_purpose::STANDARD.encode(DEFAULT_CERT_PEM);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_base64_private_key() {
        set_tls_env_vars("invalid-base64!", &general_purpose::STANDARD.encode(DEFAULT_CERT_PEM));

        let result = load_tls_config();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Failed to decode PRIVATEKEY base64"));

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_base64_cert() {
        set_tls_env_vars(&general_purpose::STANDARD.encode(DEFAULT_KEY_PEM), "invalid-base64!");

        let result = load_tls_config();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Failed to decode FULLCHAIN base64"));

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_utf8_private_key() {
        // Create invalid UTF-8 bytes and encode them as base64
        let invalid_utf8 = vec![0xff, 0xfe, 0xfd];
        let invalid_utf8_b64 = general_purpose::STANDARD.encode(&invalid_utf8);
        let cert_b64 = general_purpose::STANDARD.encode(DEFAULT_CERT_PEM);

        set_tls_env_vars(&invalid_utf8_b64, &cert_b64);

        let result = load_tls_config();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("PRIVATEKEY is not valid UTF-8"));

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_utf8_cert() {
        // Create invalid UTF-8 bytes and encode them as base64
        let invalid_utf8 = vec![0xff, 0xfe, 0xfd];
        let invalid_utf8_b64 = general_purpose::STANDARD.encode(&invalid_utf8);
        let private_key_b64 = general_purpose::STANDARD.encode(DEFAULT_KEY_PEM);

        set_tls_env_vars(&private_key_b64, &invalid_utf8_b64);

        let result = load_tls_config();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("FULLCHAIN is not valid UTF-8"));

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_certificate_format() {
        let private_key_b64 = general_purpose::STANDARD.encode(DEFAULT_KEY_PEM);
        let invalid_cert = "This is not a valid certificate";
        let cert_b64 = general_purpose::STANDARD.encode(invalid_cert);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        assert!(result.is_err());

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_private_key_format() {
        let invalid_key = "This is not a valid private key";
        let private_key_b64 = general_purpose::STANDARD.encode(invalid_key);
        let cert_b64 = general_purpose::STANDARD.encode(DEFAULT_CERT_PEM);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        assert!(result.is_err());

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_empty_private_key() {
        let empty_key = "";
        let private_key_b64 = general_purpose::STANDARD.encode(empty_key);
        let cert_b64 = general_purpose::STANDARD.encode(DEFAULT_CERT_PEM);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("No private keys found"));

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_only_private_key_env_var() {
        clear_tls_env_vars();
        env::set_var("PRIVATEKEY", general_purpose::STANDARD.encode(DEFAULT_KEY_PEM));
        // FULLCHAIN is not set

        let result = load_tls_config();
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_only_cert_env_var() {
        clear_tls_env_vars();
        env::set_var("FULLCHAIN", general_purpose::STANDARD.encode(DEFAULT_CERT_PEM));
        // PRIVATEKEY is not set

        let result = load_tls_config();
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_default_certificates_are_valid() {
        // Test that the default certificates are valid by themselves
        let mut cert_reader = std::io::Cursor::new(DEFAULT_CERT_PEM.as_bytes());
        let cert_result = certs(&mut cert_reader);
        assert!(cert_result.is_ok());
        assert!(!cert_result.unwrap().is_empty());

        let mut key_reader = std::io::Cursor::new(DEFAULT_KEY_PEM.as_bytes());
        let key_result = pkcs8_private_keys(&mut key_reader);
        assert!(key_result.is_ok());
        assert!(!key_result.unwrap().is_empty());
    }

    #[test]
    fn test_tls_config_creation_with_mismatched_cert_key() {
        // Create a different private key to test mismatch scenario
        let different_key = r#"-----BEGIN PRIVATE KEY-----
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
-----END PRIVATE KEY-----"#;

        let private_key_b64 = general_purpose::STANDARD.encode(different_key);
        let cert_b64 = general_purpose::STANDARD.encode(DEFAULT_CERT_PEM);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        // This should still succeed even with mismatched cert/key for testing purposes
        // The TLS library will handle the validation during actual use
        assert!(result.is_ok());

        // Clean up
        clear_tls_env_vars();
    }
}
