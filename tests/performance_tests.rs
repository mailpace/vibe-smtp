use std::time::{Duration, Instant};
use tokio::time::sleep;
use lettre::{Message, Transport};
use lettre::transport::smtp::authentication::Credentials;
use anyhow::Result;
use futures::future::join_all;

mod common;
use common::{TestServer, create_smtp_transport};

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
                Some(Credentials::new("test-token".to_string(), "test-token".to_string()))
            );
            
            let email = Message::builder()
                .from(format!("test{}@example.com", i).parse().unwrap())
                .to("recipient@example.com".parse().unwrap())
                .subject(format!("Test Subject {}", i))
                .body(format!("Test message body {}", i))
                .unwrap();
            
            transport.send(&email)
        })
    });
    
    // Wait for all tasks to complete
    let results = join_all(tasks).await;
    
    let duration = start_time.elapsed();
    
    // Verify all emails were sent successfully
    let successful_sends = results.into_iter()
        .filter_map(|task_result| task_result.ok())
        .filter(|send_result| send_result.is_ok())
        .count();
    
    assert_eq!(successful_sends, num_emails);
    
    // Performance assertion: should complete within reasonable time
    assert!(duration < Duration::from_secs(30), "Concurrent sends took too long: {:?}", duration);
    
    println!("Sent {} emails concurrently in {:?}", num_emails, duration);
    
    Ok(())
}

#[tokio::test]
async fn test_throughput_measurement() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;
    
    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new("test-token".to_string(), "test-token".to_string()))
    );
    
    let num_emails = 50;
    let start_time = Instant::now();
    
    // Send emails sequentially to measure throughput
    for i in 0..num_emails {
        let email = Message::builder()
            .from(format!("test{}@example.com", i).parse()?)
            .to("recipient@example.com".parse()?)
            .subject(format!("Throughput Test {}", i))
            .body(format!("Test message body {}", i))?;
        
        let result = transport.send(&email);
        assert!(result.is_ok(), "Email {} should be sent successfully", i);
    }
    
    let duration = start_time.elapsed();
    let throughput = num_emails as f64 / duration.as_secs_f64();
    
    println!("Throughput: {:.2} emails/second", throughput);
    
    // Performance assertion: should handle at least 5 emails per second
    assert!(throughput > 5.0, "Throughput too low: {:.2} emails/second", throughput);
    
    Ok(())
}

#[tokio::test]
async fn test_large_email_performance() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;
    
    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new("test-token".to_string(), "test-token".to_string()))
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
    
    println!("Large email (1MB) sent in {:?}", duration);
    
    // Performance assertion: should handle 1MB email within 10 seconds
    assert!(duration < Duration::from_secs(10), "Large email took too long: {:?}", duration);
    
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
                Some(Credentials::new("test-token".to_string(), "test-token".to_string()))
            );
            
            let email = Message::builder()
                .from(format!("test{}@example.com", i).parse().unwrap())
                .to("recipient@example.com".parse().unwrap())
                .subject(format!("Load Test {}", i))
                .body(format!("Load test message {}", i))
                .unwrap();
            
            transport.send(&email)
        })
    });
    
    let results = join_all(tasks).await;
    let duration = start_time.elapsed();
    
    // Verify all connections were handled successfully
    let successful_sends = results.into_iter()
        .filter_map(|task_result| task_result.ok())
        .filter(|send_result| send_result.is_ok())
        .count();
    
    assert_eq!(successful_sends, num_connections);
    
    println!("Handled {} concurrent connections in {:?}", num_connections, duration);
    
    // Performance assertion: should handle multiple connections efficiently
    assert!(duration < Duration::from_secs(60), "Connection handling took too long: {:?}", duration);
    
    Ok(())
}

#[tokio::test]
async fn test_memory_usage_with_attachments() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;
    
    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new("test-token".to_string(), "test-token".to_string()))
    );
    
    // Create email with multiple attachments
    let attachment_size = 100 * 1024; // 100KB per attachment
    let num_attachments = 5;
    
    let start_time = Instant::now();
    
    let mut multipart = lettre::message::MultiPart::mixed()
        .singlepart(lettre::message::SinglePart::plain("Email with multiple attachments".to_string()));
    
    for i in 0..num_attachments {
        let attachment_content = format!("Attachment {} content: {}", i, "X".repeat(attachment_size));
        let attachment = lettre::message::Attachment::new(format!("attachment_{}.txt", i))
            .body(attachment_content, lettre::message::header::ContentType::TEXT_PLAIN);
        multipart = multipart.singlepart(attachment);
    }
    
    let email = Message::builder()
        .from("test@example.com".parse()?)
        .to("recipient@example.com".parse()?)
        .subject("Memory Usage Test")
        .multipart(multipart)?;
    
    let result = transport.send(&email);
    assert!(result.is_ok(), "Email with attachments should be sent successfully");
    
    let duration = start_time.elapsed();
    
    println!("Email with {} attachments ({}KB each) sent in {:?}", 
             num_attachments, attachment_size / 1024, duration);
    
    // Performance assertion: should handle attachments efficiently
    assert!(duration < Duration::from_secs(30), "Attachment processing took too long: {:?}", duration);
    
    Ok(())
}

#[tokio::test]
async fn test_stress_test_rapid_emails() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;
    
    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new("test-token".to_string(), "test-token".to_string()))
    );
    
    let num_emails = 100;
    let start_time = Instant::now();
    
    // Send emails as fast as possible
    for i in 0..num_emails {
        let email = Message::builder()
            .from(format!("stress{}@example.com", i).parse()?)
            .to("recipient@example.com".parse()?)
            .subject(format!("Stress Test {}", i))
            .body(format!("Stress test message {}", i))?;
        
        let result = transport.send(&email);
        assert!(result.is_ok(), "Stress test email {} should be sent successfully", i);
        
        // Small delay to prevent overwhelming the system
        sleep(Duration::from_millis(10)).await;
    }
    
    let duration = start_time.elapsed();
    let throughput = num_emails as f64 / duration.as_secs_f64();
    
    println!("Stress test: {} emails in {:?} ({:.2} emails/second)", 
             num_emails, duration, throughput);
    
    // Performance assertion: should handle rapid emails
    assert!(duration < Duration::from_secs(120), "Stress test took too long: {:?}", duration);
    
    Ok(())
}
