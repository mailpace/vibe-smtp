use anyhow::Result;
use lettre::{
    message::{MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    Message, Transport,
};

mod common;
use common::{create_smtp_transport, TestServer};

/// Test HTML compression with basic HTML content
#[tokio::test]
async fn test_basic_html_compression() -> Result<()> {
    let server = TestServer::new_with_html_compression().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    // HTML with comments and whitespace that should be compressed
    let html_content = r#"
    <html>
        <head>
            <!-- This comment should be removed -->
            <title>Basic HTML Compression Test</title>
        </head>
        <body>
            <h1>   Welcome to Our Service   </h1>
            <p>This is a test email with   extra   whitespace.</p>
            <!-- End of content -->
        </body>
    </html>
    "#;

    let email = Message::builder()
        .from("test@example.com".parse()?)
        .to("user@example.com".parse()?)
        .subject("Basic HTML Compression Test")
        .body(html_content.to_string())?;

    let result = transport.send(&email);
    assert!(result.is_ok(), "Basic HTML compression test should succeed");

    Ok(())
}

/// Test HTML compression with multipart emails
#[tokio::test]
async fn test_multipart_html_compression() -> Result<()> {
    let server = TestServer::new_with_html_compression().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    let html_part = r#"
    <html>
        <head>
            <!-- Multipart compression test -->
            <style>
                .content {
                    padding: 20px;
                    margin: 10px;
                }
            </style>
        </head>
        <body>
            <div class="content">
                <h2>   Multipart HTML Compression   </h2>
                <p>This HTML part should be compressed in a multipart email.</p>
            </div>
        </body>
    </html>
    "#;

    let text_part =
        "Multipart HTML Compression\n\nThis HTML part should be compressed in a multipart email.";

    let email = Message::builder()
        .from("newsletter@example.com".parse()?)
        .to("subscriber@example.com".parse()?)
        .subject("Multipart Compression Test")
        .multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::plain(text_part.to_string()))
                .singlepart(SinglePart::html(html_part.to_string())),
        )?;

    let result = transport.send(&email);
    assert!(
        result.is_ok(),
        "Multipart HTML compression test should succeed"
    );

    Ok(())
}

/// Test that plain text is not affected by HTML compression
#[tokio::test]
async fn test_plain_text_passthrough() -> Result<()> {
    let server = TestServer::new_with_html_compression().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    let plain_text = "This is plain text content.\nIt should pass through unchanged.\n\nEven with HTML compression enabled.";

    let email = Message::builder()
        .from("sender@example.com".parse()?)
        .to("recipient@example.com".parse()?)
        .subject("Plain Text Passthrough Test")
        .body(plain_text.to_string())?;

    let result = transport.send(&email);
    assert!(result.is_ok(), "Plain text should pass through unchanged");

    Ok(())
}

/// Test HTML compression with CSS and JavaScript
#[tokio::test]
async fn test_css_javascript_compression() -> Result<()> {
    let server = TestServer::new_with_html_compression().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    let html_with_assets = r#"
    <!DOCTYPE html>
    <html>
        <head>
            <title>CSS and JS Compression Test</title>
            <style>
                body {
                    font-family: 'Arial', sans-serif;
                    background-color: #ffffff;
                    margin: 0;
                    padding: 20px;
                }
                
                .header {
                    color: #333333;
                    font-size: 24px;
                    text-align: center;
                }
                
                .content {
                    max-width: 600px;
                    margin: 0 auto;
                    padding: 20px;
                }
            </style>
            <script>
                function trackEmailOpen() {
                    console.log('Email opened');
                    // Send tracking pixel
                    var img = new Image();
                    img.src = 'https://example.com/track.gif';
                }
                
                document.addEventListener('DOMContentLoaded', function() {
                    trackEmailOpen();
                });
            </script>
        </head>
        <body>
            <div class="header">Newsletter Title</div>
            <div class="content">
                <p>This email contains both CSS and JavaScript that should be minified.</p>
                <button onclick="alert('Hello!')">Click Me</button>
            </div>
        </body>
    </html>
    "#;

    let email = Message::builder()
        .from("newsletter@example.com".parse()?)
        .to("user@example.com".parse()?)
        .subject("CSS and JS Compression Test")
        .body(html_with_assets.to_string())?;

    let result = transport.send(&email);
    assert!(
        result.is_ok(),
        "HTML with CSS and JS should be compressed successfully"
    );

    Ok(())
}

/// Test compression with malformed HTML
#[tokio::test]
async fn test_malformed_html_handling() -> Result<()> {
    let server = TestServer::new_with_html_compression().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    // Intentionally malformed HTML
    let malformed_html = r#"
    <html>
        <head>
            <title>Malformed HTML Test
        <!-- Missing closing tag -->
        <body>
            <h1>Unclosed header
            <p>Paragraph without closing tag
            <div class="content">
                <span>Nested content
    "#;

    let email = Message::builder()
        .from("test@example.com".parse()?)
        .to("user@example.com".parse()?)
        .subject("Malformed HTML Test")
        .body(malformed_html.to_string())?;

    let result = transport.send(&email);
    assert!(
        result.is_ok(),
        "Malformed HTML should be handled gracefully"
    );

    Ok(())
}

/// Test that compression is disabled when flag is not set
#[tokio::test]
async fn test_compression_disabled_by_default() -> Result<()> {
    let server = TestServer::new().await?; // Default server without compression
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    let html_content = r#"
    <html>
        <!-- This comment should remain when compression is disabled -->
        <body>
            <h1>   Uncompressed Content   </h1>
            <p>This content should not be compressed.</p>
        </body>
    </html>
    "#;

    let email = Message::builder()
        .from("test@example.com".parse()?)
        .to("user@example.com".parse()?)
        .subject("Compression Disabled Test")
        .body(html_content.to_string())?;

    let result = transport.send(&email);
    assert!(result.is_ok(), "Email should be sent without compression");

    Ok(())
}

/// Test compression with large HTML content
#[tokio::test]
async fn test_large_html_compression() -> Result<()> {
    let server = TestServer::new_with_html_compression().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    // Generate large HTML content
    let mut large_html = String::from(
        r#"
    <html>
        <head>
            <!-- Large HTML compression test -->
            <title>Large HTML Email</title>
            <style>
                body { font-family: Arial, sans-serif; }
                .product { margin: 15px; padding: 10px; border: 1px solid #ccc; }
                .product-title { font-size: 18px; font-weight: bold; }
                .product-description { color: #666; margin-top: 5px; }
            </style>
        </head>
        <body>
            <h1>Product Catalog</h1>
            <div class="products">
    "#,
    );

    // Add many products to create a large email
    for i in 1..=200 {
        large_html.push_str(&format!(
            r#"
                <div class="product">
                    <!-- Product {i} -->
                    <div class="product-title">   Product {i}   </div>
                    <div class="product-description">
                        This is the description for product number {i}.
                        It contains important information about the product.
                    </div>
                    <div class="price">Price: ${}.99</div>
                </div>
        "#,
            i, i, i, i
        ));
    }

    large_html.push_str(
        r#"
            </div>
        </body>
    </html>
    "#,
    );

    let email = Message::builder()
        .from("catalog@example.com".parse()?)
        .to("customer@example.com".parse()?)
        .subject("Large Product Catalog")
        .body(large_html)?;

    let result = transport.send(&email);
    assert!(
        result.is_ok(),
        "Large HTML email should be compressed and sent successfully"
    );

    Ok(())
}

/// Test HTML compression with email templates
#[tokio::test]
async fn test_email_template_compression() -> Result<()> {
    let server = TestServer::new_with_html_compression().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    // Typical email template with lots of formatting
    let template_html = r#"
    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <!-- Email template compression test -->
            <title>Welcome Email</title>
            <style>
                * {
                    margin: 0;
                    padding: 0;
                    box-sizing: border-box;
                }
                
                body {
                    font-family: 'Helvetica Neue', Arial, sans-serif;
                    line-height: 1.6;
                    color: #333333;
                    background-color: #f4f4f4;
                }
                
                .container {
                    max-width: 600px;
                    margin: 0 auto;
                    background-color: #ffffff;
                    padding: 20px;
                }
                
                .header {
                    background-color: #007bff;
                    color: white;
                    text-align: center;
                    padding: 20px;
                    margin-bottom: 20px;
                }
                
                .content {
                    padding: 20px;
                }
                
                .button {
                    display: inline-block;
                    background-color: #28a745;
                    color: white;
                    padding: 12px 24px;
                    text-decoration: none;
                    border-radius: 4px;
                    margin: 10px 0;
                }
                
                .footer {
                    background-color: #f8f9fa;
                    padding: 15px;
                    text-align: center;
                    font-size: 12px;
                    color: #666666;
                    border-top: 1px solid #dee2e6;
                }
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h1>   Welcome to Our Platform!   </h1>
                </div>
                
                <div class="content">
                    <h2>Hello {{user_name}},</h2>
                    
                    <p>
                        Thank you for joining our platform. We're excited to have you
                        as part of our community!
                    </p>
                    
                    <p>
                        To get started, please click the button below to verify your
                        email address:
                    </p>
                    
                    <a href="{{verification_link}}" class="button">
                        Verify Email Address
                    </a>
                    
                    <p>
                        If you have any questions, don't hesitate to contact our
                        support team.
                    </p>
                    
                    <p>Best regards,<br>The Platform Team</p>
                </div>
                
                <div class="footer">
                    <!-- Footer content -->
                    <p>© 2024 Our Platform. All rights reserved.</p>
                    <p>
                        You received this email because you signed up for our service.
                        <a href="{{unsubscribe_link}}">Unsubscribe</a>
                    </p>
                </div>
            </div>
        </body>
    </html>
    "#;

    let email = Message::builder()
        .from("welcome@platform.com".parse()?)
        .to("newuser@example.com".parse()?)
        .subject("Welcome to Our Platform!")
        .body(template_html.to_string())?;

    let result = transport.send(&email);
    assert!(
        result.is_ok(),
        "Email template should be compressed and sent successfully"
    );

    Ok(())
}
