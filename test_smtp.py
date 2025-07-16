#!/usr/bin/env python3
"""
Simple test script to send an email through the SMTP gateway.
This demonstrates how to use the server with various features.
"""

import smtplib
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
from email.mime.base import MIMEBase
from email import encoders
import sys
import os

def send_test_email():
    # SMTP server configuration
    smtp_host = "localhost"
    smtp_port = 2525
    
    # Email configuration
    sender = "test@yourdomain.com"
    recipient = "recipient@example.com"
    
    # MailPace API token (you should set this)
    api_token = os.environ.get("MAILPACE_API_TOKEN", "your_api_token_here")
    
    # Create message
    msg = MIMEMultipart()
    msg['From'] = sender
    msg['To'] = recipient
    msg['Subject'] = "Test Email from Vibe Gateway"
    
    # Add MailPace-specific headers
    msg['X-MailPace-Tags'] = "test, gateway, demo"
    msg['X-List-Unsubscribe'] = "<mailto:unsubscribe@yourdomain.com?subject=unsubscribe>"
    
    # Add body
    html_body = """
    <html>
    <body>
    <h1>Test Email</h1>
    <p>This is a test email sent through the Vibe Gateway SMTP server.</p>
    <p>It demonstrates:</p>
    <ul>
        <li>HTML email content</li>
        <li>MailPace tags</li>
        <li>List-Unsubscribe header</li>
        <li>Authentication with MailPace API token</li>
    </ul>
    </body>
    </html>
    """
    
    text_body = """
    Test Email
    
    This is a test email sent through the Vibe Gateway SMTP server.
    
    It demonstrates:
    - Text email content
    - MailPace tags
    - List-Unsubscribe header
    - Authentication with MailPace API token
    """
    
    msg.attach(MIMEText(text_body, 'plain'))
    msg.attach(MIMEText(html_body, 'html'))
    
    # Optional: Add a small text attachment
    if len(sys.argv) > 1 and sys.argv[1] == "--with-attachment":
        attachment_content = "This is a test attachment."
        attachment = MIMEBase('text', 'plain')
        attachment.set_payload(attachment_content.encode())
        encoders.encode_base64(attachment)
        attachment.add_header(
            'Content-Disposition',
            'attachment; filename="test.txt"'
        )
        msg.attach(attachment)
    
    try:
        # Connect and send
        server = smtplib.SMTP(smtp_host, smtp_port)
        server.set_debuglevel(1)  # Enable debug output
        
        # Authenticate with MailPace API token as both username and password
        server.login(api_token, api_token)
        
        # Send email
        server.send_message(msg)
        server.quit()
        
        print("Email sent successfully!")
        
    except Exception as e:
        print(f"Error sending email: {e}")
        return False
    
    return True

if __name__ == "__main__":
    print("Testing Vibe Gateway SMTP Server...")
    print("Make sure the server is running with: cargo run")
    print("Set your MailPace API token: export MAILPACE_API_TOKEN=your_token_here")
    print()
    
    if send_test_email():
        print("Test completed successfully!")
    else:
        print("Test failed!")
        sys.exit(1)
