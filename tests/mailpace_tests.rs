use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use vibe_gateway::mailpace::{Attachment, MailPaceClient, MailPacePayload};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn test_mailpace_client_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/send"))
        .and(header("Content-Type", "application/json"))
        .and(header("MailPace-Server-Token", "test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_123",
            "status": "sent"
        })))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let mailpace_client = MailPaceClient::new(
        client,
        format!("{}/api/v1/send", mock_server.uri()),
        2,
        std::time::Duration::from_millis(250),
    );

    let payload = MailPacePayload {
        from: "test@example.com".to_string(),
        to: "recipient@example.com".to_string(),
        cc: None,
        bcc: None,
        subject: Some("Test Subject".to_string()),
        htmlbody: Some("<h1>Test</h1>".to_string()),
        textbody: Some("Test".to_string()),
        replyto: None,
        list_unsubscribe: None,
        attachments: None,
        tags: Some(vec!["test".to_string()]),
    };

    let result = mailpace_client.send_email(&payload, "test-token").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mailpace_client_with_attachments() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_123",
            "status": "sent"
        })))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let mailpace_client = MailPaceClient::new(
        client,
        format!("{}/api/v1/send", mock_server.uri()),
        2,
        std::time::Duration::from_millis(250),
    );

    let payload = MailPacePayload {
        from: "test@example.com".to_string(),
        to: "recipient@example.com".to_string(),
        cc: None,
        bcc: None,
        subject: Some("Test with Attachment".to_string()),
        htmlbody: None,
        textbody: Some("Test with attachment".to_string()),
        replyto: None,
        list_unsubscribe: None,
        attachments: Some(vec![Attachment {
            name: "test.txt".to_string(),
            content: STANDARD.encode("Test content"),
            content_type: "text/plain".to_string(),
            cid: None,
        }]),
        tags: None,
    };

    let result = mailpace_client.send_email(&payload, "test-token").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mailpace_client_error_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/send"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "errors": ["Invalid email format"]
        })))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let mailpace_client = MailPaceClient::new(
        client,
        format!("{}/api/v1/send", mock_server.uri()),
        2,
        std::time::Duration::from_millis(250),
    );

    let payload = MailPacePayload {
        from: "invalid-email".to_string(),
        to: "recipient@example.com".to_string(),
        cc: None,
        bcc: None,
        subject: Some("Test Subject".to_string()),
        htmlbody: None,
        textbody: Some("Test".to_string()),
        replyto: None,
        list_unsubscribe: None,
        attachments: None,
        tags: None,
    };

    let result = mailpace_client.send_email(&payload, "test-token").await;
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("400"));
}

#[tokio::test]
async fn test_mailpace_client_network_error() {
    let client = reqwest::Client::new();
    let mailpace_client = MailPaceClient::new(
        client,
        "http://non-existent-domain.invalid/api/v1/send".to_string(),
        2,
        std::time::Duration::from_millis(250),
    );

    let payload = MailPacePayload {
        from: "test@example.com".to_string(),
        to: "recipient@example.com".to_string(),
        cc: None,
        bcc: None,
        subject: Some("Test Subject".to_string()),
        htmlbody: None,
        textbody: Some("Test".to_string()),
        replyto: None,
        list_unsubscribe: None,
        attachments: None,
        tags: None,
    };

    let result = mailpace_client.send_email(&payload, "test-token").await;
    assert!(result.is_err());
}

#[test]
fn test_attachment_serialization() {
    let attachment = Attachment {
        name: "test.txt".to_string(),
        content: "VGVzdCBjb250ZW50".to_string(), // "Test content" in base64
        content_type: "text/plain".to_string(),
        cid: Some("cid123".to_string()),
    };

    let serialized = serde_json::to_string(&attachment).unwrap();
    let expected = r#"{"name":"test.txt","content":"VGVzdCBjb250ZW50","content_type":"text/plain","cid":"cid123"}"#;
    assert_eq!(serialized, expected);
}

#[test]
fn test_mailpace_payload_serialization() {
    let payload = MailPacePayload {
        from: "test@example.com".to_string(),
        to: "recipient@example.com".to_string(),
        cc: Some("cc@example.com".to_string()),
        bcc: None,
        subject: Some("Test Subject".to_string()),
        htmlbody: Some("<h1>Test</h1>".to_string()),
        textbody: Some("Test".to_string()),
        replyto: Some("reply@example.com".to_string()),
        list_unsubscribe: Some("<mailto:unsubscribe@example.com>".to_string()),
        attachments: None,
        tags: Some(vec!["test".to_string(), "unit".to_string()]),
    };

    let serialized = serde_json::to_value(&payload).unwrap();

    assert_eq!(serialized["from"], "test@example.com");
    assert_eq!(serialized["to"], "recipient@example.com");
    assert_eq!(serialized["cc"], "cc@example.com");
    assert_eq!(serialized["subject"], "Test Subject");
    assert_eq!(serialized["htmlbody"], "<h1>Test</h1>");
    assert_eq!(serialized["textbody"], "Test");
    assert_eq!(serialized["replyto"], "reply@example.com");
    assert_eq!(
        serialized["list_unsubscribe"],
        "<mailto:unsubscribe@example.com>"
    );
    assert_eq!(serialized["tags"], json!(["test", "unit"]));

    // bcc should not be present when None
    assert!(serialized.get("bcc").is_none());
}

#[test]
fn test_mailpace_payload_optional_fields() {
    let payload = MailPacePayload {
        from: "test@example.com".to_string(),
        to: "recipient@example.com".to_string(),
        cc: None,
        bcc: None,
        subject: None,
        htmlbody: None,
        textbody: None,
        replyto: None,
        list_unsubscribe: None,
        attachments: None,
        tags: None,
    };

    let serialized = serde_json::to_value(&payload).unwrap();

    assert_eq!(serialized["from"], "test@example.com");
    assert_eq!(serialized["to"], "recipient@example.com");

    // Optional fields should not be present
    assert!(serialized.get("cc").is_none());
    assert!(serialized.get("bcc").is_none());
    assert!(serialized.get("subject").is_none());
    assert!(serialized.get("htmlbody").is_none());
    assert!(serialized.get("textbody").is_none());
    assert!(serialized.get("replyto").is_none());
    assert!(serialized.get("list_unsubscribe").is_none());
    assert!(serialized.get("attachments").is_none());
    assert!(serialized.get("tags").is_none());
}
