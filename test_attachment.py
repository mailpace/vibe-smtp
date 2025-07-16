#!/usr/bin/env python3
"""
Test script to verify attachment support in vibe-gateway
"""
import smtplib
import base64
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
from email.mime.base import MIMEBase
from email import encoders
import os

def test_attachment_support():
    """Test sending an email with attachment through vibe-gateway"""
    
    # Create a simple test file
    test_content = b"This is a test attachment content.\nLine 2 of the test file."
    
    # Create message
    msg = MIMEMultipart()
    msg['From'] = 'sender@example.com'
    msg['To'] = 'recipient@example.com'
    msg['Subject'] = 'Test Email with Attachment'
    
    # Add body
    body = "This is a test email with an attachment."
    msg.attach(MIMEText(body, 'plain'))
    
    # Add attachment
    part = MIMEBase('application', 'octet-stream')
    part.set_payload(test_content)
    encoders.encode_base64(part)
    part.add_header(
        'Content-Disposition',
        'attachment; filename="test.txt"'
    )
    msg.attach(part)
    
    # Send email
    try:
        server = smtplib.SMTP('localhost', 2525)
        server.set_debuglevel(1)  # Enable debug output
        
        # If you have a MailPace API token, you can authenticate
        # server.login('your_api_token', 'your_api_token')
        
        text = msg.as_string()
        server.sendmail('sender@example.com', ['recipient@example.com'], text)
        server.quit()
        
        print("Email with attachment sent successfully!")
        
    except Exception as e:
        print(f"Error sending email: {e}")

if __name__ == "__main__":
    test_attachment_support()
