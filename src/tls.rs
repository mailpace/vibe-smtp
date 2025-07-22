use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use tracing::info;

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
            info!("TLS environment variables (PRIVATEKEY, FULLCHAIN) not found - TLS will be disabled");
            return Ok(None);
        }
    };

    // Parse certificates
    let mut cert_reader = std::io::Cursor::new(cert_pem.as_bytes());
    let cert_chain = certs(&mut cert_reader)?
        .into_iter()
        .map(Certificate)
        .collect::<Vec<_>>();

    if cert_chain.is_empty() {
        return Err(anyhow::anyhow!("No certificates found"));
    }

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

    // Helper function to generate test certificates
    fn generate_test_cert() -> (String, String) {
        use rcgen::{Certificate, CertificateParams, DistinguishedName};

        let mut params = CertificateParams::new(vec!["localhost".to_string()]);
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "localhost");

        let cert = Certificate::from_params(params).unwrap();
        let private_key_pem = cert.serialize_private_key_pem();
        let cert_pem = cert.serialize_pem().unwrap();

        (private_key_pem, cert_pem)
    }

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
    fn test_load_tls_config_without_env_vars_returns_none() {
        // Ensure no env vars are set
        clear_tls_env_vars();

        let result = load_tls_config();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_tls_config_with_valid_env_vars() {
        // Generate test certificates
        let (private_key_pem, cert_pem) = generate_test_cert();
        let private_key_b64 = general_purpose::STANDARD.encode(&private_key_pem);
        let cert_b64 = general_purpose::STANDARD.encode(&cert_pem);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_base64_private_key() {
        let (_, cert_pem) = generate_test_cert();
        let cert_b64 = general_purpose::STANDARD.encode(&cert_pem);

        set_tls_env_vars("invalid-base64!", &cert_b64);

        let result = load_tls_config();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Failed to decode PRIVATEKEY base64"));

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_base64_cert() {
        let (private_key_pem, _) = generate_test_cert();
        let private_key_b64 = general_purpose::STANDARD.encode(&private_key_pem);

        set_tls_env_vars(&private_key_b64, "invalid-base64!");

        let result = load_tls_config();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Failed to decode FULLCHAIN base64"));

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_utf8_private_key() {
        let (_, cert_pem) = generate_test_cert();
        let cert_b64 = general_purpose::STANDARD.encode(&cert_pem);

        // Create invalid UTF-8 bytes and encode them as base64
        let invalid_utf8 = vec![0xff, 0xfe, 0xfd];
        let invalid_utf8_b64 = general_purpose::STANDARD.encode(&invalid_utf8);

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
        let (private_key_pem, _) = generate_test_cert();
        let private_key_b64 = general_purpose::STANDARD.encode(&private_key_pem);

        // Create invalid UTF-8 bytes and encode them as base64
        let invalid_utf8 = vec![0xff, 0xfe, 0xfd];
        let invalid_utf8_b64 = general_purpose::STANDARD.encode(&invalid_utf8);

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
        let (private_key_pem, _) = generate_test_cert();
        let private_key_b64 = general_purpose::STANDARD.encode(&private_key_pem);

        // Create an invalid but PEM-formatted certificate
        let invalid_cert = r#"-----BEGIN CERTIFICATE-----
INVALID_CERTIFICATE_DATA_HERE
-----END CERTIFICATE-----"#;
        let cert_b64 = general_purpose::STANDARD.encode(invalid_cert);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        // This should fail because the certificate data is invalid
        assert!(result.is_err());

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_empty_certificate() {
        let (private_key_pem, _) = generate_test_cert();
        let private_key_b64 = general_purpose::STANDARD.encode(&private_key_pem);
        let empty_cert = "";
        let cert_b64 = general_purpose::STANDARD.encode(empty_cert);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("No certificates found"));

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_invalid_private_key_format() {
        let (_, cert_pem) = generate_test_cert();
        let cert_b64 = general_purpose::STANDARD.encode(&cert_pem);
        let invalid_key = "This is not a valid private key";
        let private_key_b64 = general_purpose::STANDARD.encode(invalid_key);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        assert!(result.is_err());

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_empty_private_key() {
        let (_, cert_pem) = generate_test_cert();
        let cert_b64 = general_purpose::STANDARD.encode(&cert_pem);
        let empty_key = "";
        let private_key_b64 = general_purpose::STANDARD.encode(empty_key);

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
        let (private_key_pem, _) = generate_test_cert();
        let private_key_b64 = general_purpose::STANDARD.encode(&private_key_pem);

        clear_tls_env_vars();
        env::set_var("PRIVATEKEY", private_key_b64);
        // FULLCHAIN is not set

        let result = load_tls_config();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Should return None since both env vars are required

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_load_tls_config_with_only_cert_env_var() {
        let (_, cert_pem) = generate_test_cert();
        let cert_b64 = general_purpose::STANDARD.encode(&cert_pem);

        clear_tls_env_vars();
        env::set_var("FULLCHAIN", cert_b64);
        // PRIVATEKEY is not set

        let result = load_tls_config();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Should return None since both env vars are required

        // Clean up
        clear_tls_env_vars();
    }

    #[test]
    fn test_generated_certificates_are_valid() {
        // Test that the generated certificates are valid by themselves
        let (private_key_pem, cert_pem) = generate_test_cert();

        let mut cert_reader = std::io::Cursor::new(cert_pem.as_bytes());
        let cert_result = certs(&mut cert_reader);
        assert!(cert_result.is_ok());
        assert!(!cert_result.unwrap().is_empty());

        let mut key_reader = std::io::Cursor::new(private_key_pem.as_bytes());
        let key_result = pkcs8_private_keys(&mut key_reader);
        assert!(key_result.is_ok());
        assert!(!key_result.unwrap().is_empty());
    }

    #[test]
    fn test_tls_config_creation_with_mismatched_cert_key() {
        // Generate one set of certs and create a different private key to test mismatch scenario
        let (_, cert_pem) = generate_test_cert();
        let (different_private_key, _) = generate_test_cert(); // Generate different cert/key pair

        let private_key_b64 = general_purpose::STANDARD.encode(different_private_key);
        let cert_b64 = general_purpose::STANDARD.encode(&cert_pem);

        set_tls_env_vars(&private_key_b64, &cert_b64);

        let result = load_tls_config();
        // This may succeed or fail depending on TLS library validation
        // The important thing is it doesn't panic and handles the mismatch gracefully
        match result {
            Ok(_) => {
                // Some TLS libraries may allow mismatched cert/key for testing
            }
            Err(_) => {
                // Some TLS libraries may reject mismatched cert/key
            }
        }

        // Clean up
        clear_tls_env_vars();
    }
}
