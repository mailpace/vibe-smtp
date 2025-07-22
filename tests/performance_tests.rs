use anyhow::Result;
use futures::future::join_all;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, Transport};
use std::time::{Duration, Instant};
use tokio::time::sleep;

mod common;
use common::{create_smtp_transport, TestServer};

#[tokio::test]
async fn test_concurrent_email_sending() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;

    let start_time = Instant::now();
    let num_emails = 10;

    // Create multiple concurrent email sending tasks
    let tasks = (0..num_emails).map(|i| {
        let server_addr = server.smtp_address();
        tokio::spawn(async move {
            let transport = create_smtp_transport(
                server_addr,
                Some(Credentials::new(
                    "test-token".to_string(),
                    "test-token".to_string(),
                )),
            );

            let email = Message::builder()
                .from(format!("test{i}@example.com").parse().unwrap())
                .to("recipient@example.com".parse().unwrap())
                .subject(format!("Test Subject {i}"))
                .body(format!("Test message body {i}"))
                .unwrap();

            transport.send(&email)
        })
    });

    // Wait for all tasks to complete
    let results = join_all(tasks).await;

    let duration = start_time.elapsed();

    // Verify all emails were sent successfully
    let successful_sends = results
        .into_iter()
        .filter_map(|task_result| task_result.ok())
        .filter(|send_result| send_result.is_ok())
        .count();

    assert_eq!(successful_sends, num_emails);

    // Performance assertion: should complete within reasonable time
    assert!(
        duration < Duration::from_secs(30),
        "Concurrent sends took too long: {duration:?}"
    );

    println!("Sent {num_emails} emails concurrently in {duration:?}");

    Ok(())
}

#[tokio::test]
async fn test_throughput_measurement() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    let num_emails = 50;
    let start_time = Instant::now();

    // Send emails sequentially to measure throughput
    for i in 0..num_emails {
        let email = Message::builder()
            .from(format!("test{i}@example.com").parse()?)
            .to("recipient@example.com".parse()?)
            .subject(format!("Throughput Test {i}"))
            .body(format!("Test message body {i}"))?;

        let result = transport.send(&email);
        assert!(result.is_ok(), "Email {i} should be sent successfully");
    }

    let duration = start_time.elapsed();
    let throughput = num_emails as f64 / duration.as_secs_f64();

    println!("Throughput: {throughput:.2} emails/second");

    // Performance assertion: should handle at least 5 emails per second
    assert!(
        throughput > 5.0,
        "Throughput too low: {throughput:.2} emails/second"
    );

    Ok(())
}

#[tokio::test]
async fn test_large_email_performance() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    // Create a large email (1MB)
    let large_body = "A".repeat(1024 * 1024);

    let start_time = Instant::now();

    let email = Message::builder()
        .from("test@example.com".parse()?)
        .to("recipient@example.com".parse()?)
        .subject("Large Email Performance Test")
        .body(large_body)?;

    let result = transport.send(&email);
    assert!(result.is_ok(), "Large email should be sent successfully");

    let duration = start_time.elapsed();

    println!("Large email (1MB) sent in {duration:?}");

    // Performance assertion: should handle 1MB email within 10 seconds
    assert!(
        duration < Duration::from_secs(10),
        "Large email took too long: {duration:?}"
    );

    Ok(())
}

#[tokio::test]
async fn test_connection_handling_under_load() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;

    let num_connections = 20;
    let start_time = Instant::now();

    // Create multiple connections simultaneously
    let tasks = (0..num_connections).map(|i| {
        let server_addr = server.smtp_address();
        tokio::spawn(async move {
            // Create a new connection for each task
            let transport = create_smtp_transport(
                server_addr,
                Some(Credentials::new(
                    "test-token".to_string(),
                    "test-token".to_string(),
                )),
            );

            let email = Message::builder()
                .from(format!("test{i}@example.com").parse().unwrap())
                .to("recipient@example.com".parse().unwrap())
                .subject(format!("Load Test {i}"))
                .body(format!("Load test message {i}"))
                .unwrap();

            transport.send(&email)
        })
    });

    let results = join_all(tasks).await;
    let duration = start_time.elapsed();

    // Verify all connections were handled successfully
    let successful_sends = results
        .into_iter()
        .filter_map(|task_result| task_result.ok())
        .filter(|send_result| send_result.is_ok())
        .count();

    assert_eq!(successful_sends, num_connections);

    println!("Handled {num_connections} concurrent connections in {duration:?}");

    // Performance assertion: should handle multiple connections efficiently
    assert!(
        duration < Duration::from_secs(60),
        "Connection handling took too long: {duration:?}"
    );

    Ok(())
}

#[tokio::test]
async fn test_memory_usage_with_attachments() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    // Create email with multiple attachments
    let attachment_size = 100 * 1024; // 100KB per attachment
    let num_attachments = 5;

    let start_time = Instant::now();

    let mut multipart = lettre::message::MultiPart::mixed().singlepart(
        lettre::message::SinglePart::plain("Email with multiple attachments".to_string()),
    );

    for i in 0..num_attachments {
        let attachment_content =
            format!("Attachment {} content: {}", i, "X".repeat(attachment_size));
        let attachment = lettre::message::Attachment::new(format!("attachment_{i}.txt")).body(
            attachment_content,
            lettre::message::header::ContentType::TEXT_PLAIN,
        );
        multipart = multipart.singlepart(attachment);
    }

    let email = Message::builder()
        .from("test@example.com".parse()?)
        .to("recipient@example.com".parse()?)
        .subject("Memory Usage Test")
        .multipart(multipart)?;

    let result = transport.send(&email);
    assert!(
        result.is_ok(),
        "Email with attachments should be sent successfully"
    );

    let duration = start_time.elapsed();

    println!(
        "Email with {} attachments ({}KB each) sent in {:?}",
        num_attachments,
        attachment_size / 1024,
        duration
    );

    // Performance assertion: should handle attachments efficiently
    assert!(
        duration < Duration::from_secs(30),
        "Attachment processing took too long: {duration:?}"
    );

    Ok(())
}

#[tokio::test]
async fn test_stress_test_rapid_emails() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    let num_emails = 100;
    let start_time = Instant::now();

    // Send emails as fast as possible
    for i in 0..num_emails {
        let email = Message::builder()
            .from(format!("stress{i}@example.com").parse()?)
            .to("recipient@example.com".parse()?)
            .subject(format!("Stress Test {i}"))
            .body(format!("Stress test message {i}"))?;

        let result = transport.send(&email);
        assert!(
            result.is_ok(),
            "Stress test email {i} should be sent successfully"
        );

        // Small delay to prevent overwhelming the system
        sleep(Duration::from_millis(10)).await;
    }

    let duration = start_time.elapsed();
    let throughput = num_emails as f64 / duration.as_secs_f64();

    println!("Stress test: {num_emails} emails in {duration:?} ({throughput:.2} emails/second)");

    // Performance assertion: should handle rapid emails
    assert!(
        duration < Duration::from_secs(120),
        "Stress test took too long: {duration:?}"
    );

    Ok(())
}

// HTML Compression Performance Tests
#[tokio::test]
async fn test_html_compression_performance() -> Result<()> {
    let server = TestServer::new_with_html_compression().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    // Create a moderately large HTML email for compression testing
    let html_content = generate_test_html_email(1000); // 1000 elements

    let start_time = Instant::now();
    let num_emails = 20;

    // Send multiple HTML emails to test compression performance
    let tasks = (0..num_emails).map(|i| {
        let transport = create_smtp_transport(
            server.smtp_address(),
            Some(Credentials::new(
                "test-token".to_string(),
                "test-token".to_string(),
            )),
        );
        let html_content = html_content.clone();

        tokio::spawn(async move {
            let email = Message::builder()
                .from(format!("perf-test-{i}@example.com").parse().unwrap())
                .to("recipient@example.com".parse().unwrap())
                .subject(format!("HTML Compression Performance Test {i}"))
                .body(html_content)
                .unwrap();

            transport.send(&email)
        })
    });

    let results = join_all(tasks).await;
    let duration = start_time.elapsed();

    // Verify all emails were sent successfully
    let successful_sends = results
        .into_iter()
        .filter_map(|task_result| task_result.ok())
        .filter(|send_result| send_result.is_ok())
        .count();

    assert_eq!(
        successful_sends, num_emails,
        "All HTML emails should be sent successfully"
    );

    let throughput = num_emails as f64 / duration.as_secs_f64();
    println!("HTML compression performance: {num_emails} emails in {duration:?} ({throughput:.2} emails/second)");

    // Performance assertion: compression shouldn't significantly impact performance
    assert!(
        duration < Duration::from_secs(60),
        "HTML compression performance test took too long: {duration:?}"
    );

    Ok(())
}

#[tokio::test]
async fn test_compression_vs_no_compression_performance() -> Result<()> {
    // Test with compression enabled
    let server_with_compression = TestServer::new_with_html_compression().await?;
    server_with_compression
        .mock_server
        .setup_success_response()
        .await;

    let html_content = generate_test_html_email(500);
    let num_emails = 10;

    // Test with compression
    let start_time = Instant::now();
    let tasks_with_compression = (0..num_emails).map(|i| {
        let transport = create_smtp_transport(
            server_with_compression.smtp_address(),
            Some(Credentials::new(
                "test-token".to_string(),
                "test-token".to_string(),
            )),
        );
        let html_content = html_content.clone();

        tokio::spawn(async move {
            let email = Message::builder()
                .from(format!("comp-test-{i}@example.com").parse().unwrap())
                .to("recipient@example.com".parse().unwrap())
                .subject(format!("Compression Test {i}"))
                .body(html_content)
                .unwrap();

            transport.send(&email)
        })
    });

    let _results_with_compression = join_all(tasks_with_compression).await;
    let duration_with_compression = start_time.elapsed();

    // Clean up
    drop(server_with_compression);
    sleep(Duration::from_millis(500)).await;

    // Test without compression
    let server_without_compression = TestServer::new().await?;
    server_without_compression
        .mock_server
        .setup_success_response()
        .await;

    let start_time = Instant::now();
    let tasks_without_compression = (0..num_emails).map(|i| {
        let transport = create_smtp_transport(
            server_without_compression.smtp_address(),
            Some(Credentials::new(
                "test-token".to_string(),
                "test-token".to_string(),
            )),
        );
        let html_content = html_content.clone();

        tokio::spawn(async move {
            let email = Message::builder()
                .from(format!("no-comp-test-{i}@example.com").parse().unwrap())
                .to("recipient@example.com".parse().unwrap())
                .subject(format!("No Compression Test {i}"))
                .body(html_content)
                .unwrap();

            transport.send(&email)
        })
    });

    let _results_without_compression = join_all(tasks_without_compression).await;
    let duration_without_compression = start_time.elapsed();

    println!("With compression: {duration_with_compression:?}");
    println!("Without compression: {duration_without_compression:?}");

    // Compression shouldn't add more than 50% overhead
    let overhead_ratio =
        duration_with_compression.as_secs_f64() / duration_without_compression.as_secs_f64();
    assert!(
        overhead_ratio < 1.5,
        "HTML compression adds too much overhead: {overhead_ratio:.2}x"
    );

    Ok(())
}

#[tokio::test]
async fn test_large_html_compression_performance() -> Result<()> {
    let server = TestServer::new_with_html_compression().await?;
    server.mock_server.setup_success_response().await;

    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new(
            "test-token".to_string(),
            "test-token".to_string(),
        )),
    );

    // Generate very large HTML content
    let large_html = generate_test_html_email(5000); // 5000 elements - very large

    let start_time = Instant::now();

    let email = Message::builder()
        .from("large-test@example.com".parse()?)
        .to("recipient@example.com".parse()?)
        .subject("Large HTML Compression Test")
        .body(large_html)?;

    let result = transport.send(&email);
    let duration = start_time.elapsed();

    assert!(
        result.is_ok(),
        "Large HTML email should be sent successfully"
    );

    println!("Large HTML compression: {duration:?}");

    // Even large HTML should be processed reasonably quickly
    assert!(
        duration < Duration::from_secs(30),
        "Large HTML compression took too long: {duration:?}"
    );

    Ok(())
}

/// Helper function to generate test HTML content of varying sizes
fn generate_test_html_email(num_elements: usize) -> String {
    let mut html = String::from(
        r#"
    <!DOCTYPE html>
    <html>
        <head>
            <!-- Performance test HTML -->
            <title>Performance Test Email</title>
            <style>
                body { 
                    font-family: Arial, sans-serif; 
                    margin: 0; 
                    padding: 20px; 
                }
                .item { 
                    margin: 10px 0; 
                    padding: 15px; 
                    border: 1px solid #ddd; 
                    background-color: #f9f9f9; 
                }
                .item h3 { 
                    color: #333; 
                    margin: 0 0 10px 0; 
                }
                .item p { 
                    color: #666; 
                    line-height: 1.5; 
                }
                .highlight { 
                    background-color: #fff3cd; 
                    padding: 5px; 
                    border-radius: 3px; 
                }
            </style>
        </head>
        <body>
            <h1>   Performance Test Email   </h1>
            <div class="content">
    "#,
    );

    // Add many HTML elements to test compression performance
    for i in 0..num_elements {
        html.push_str(&format!(
            r#"
                <div class="item">
                    <!-- Item {} comment with extra whitespace -->
                    <h3>   Item Number {}   </h3>
                    <p>
                        This is item number {} with some descriptive text.
                        It contains <span class="highlight">highlighted content</span> 
                        and other formatting elements to test compression.
                    </p>
                    <div class="meta">
                        <small>Created: 2024-07-{:02}</small>
                    </div>
                </div>
        "#,
            i,
            i,
            i,
            (i % 30) + 1
        ));

        // Add some variety every 50 items
        if i % 50 == 0 {
            html.push_str(&format!(
                r#"
                <div class="separator">
                    <!-- Section {} -->
                    <hr style="  margin:   20px   0  ">
                    <h2>   Section {}   </h2>
                </div>
            "#,
                i / 50,
                i / 50
            ));
        }
    }

    html.push_str(
        r#"
            </div>
            <footer style="  margin-top:   30px;   text-align:   center  ">
                <!-- Footer with lots of whitespace -->
                <p>End of performance test email</p>
            </footer>
        </body>
    </html>
    "#,
    );

    html
}
