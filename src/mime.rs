use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::mailpace::Attachment;

#[derive(Debug, Clone)]
pub struct MimeHeader {
    pub name: String,
    pub value: String,
    pub params: HashMap<String, String>,
}

impl MimeHeader {
    pub fn parse(line: &str) -> Result<Self> {
        let mut parts = line.splitn(2, ':');
        let name = parts.next().unwrap_or("").trim().to_lowercase();
        let value = parts.next().unwrap_or("").trim();
        
        // Parse parameters from value (e.g., "text/plain; charset=utf-8")
        let mut params = HashMap::new();
        let mut value_parts = value.split(';');
        let main_value = value_parts.next().unwrap_or("").trim().to_string();
        
        for param in value_parts {
            let param = param.trim();
            if let Some(eq_pos) = param.find('=') {
                let key = param[..eq_pos].trim().to_lowercase();
                let mut val = param[eq_pos + 1..].trim();
                
                // Remove quotes
                if val.starts_with('"') && val.ends_with('"') {
                    val = &val[1..val.len() - 1];
                }
                
                params.insert(key, val.to_string());
            }
        }
        
        Ok(MimeHeader {
            name,
            value: main_value,
            params,
        })
    }
    
    pub fn get_param(&self, name: &str) -> Option<&String> {
        self.params.get(name)
    }
}

#[derive(Debug)]
pub struct MimePart {
    pub headers: HashMap<String, MimeHeader>,
    pub body: Vec<u8>,
}

impl MimePart {
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }
    
    pub fn get_header(&self, name: &str) -> Option<&MimeHeader> {
        self.headers.get(&name.to_lowercase())
    }
    
    pub fn is_attachment(&self) -> bool {
        if let Some(disposition) = self.get_header("content-disposition") {
            disposition.value.starts_with("attachment")
        } else {
            false
        }
    }
    
    pub fn get_filename(&self) -> Option<String> {
        if let Some(disposition) = self.get_header("content-disposition") {
            if let Some(filename) = disposition.get_param("filename") {
                return Some(filename.clone());
            }
        }
        
        if let Some(content_type) = self.get_header("content-type") {
            if let Some(name) = content_type.get_param("name") {
                return Some(name.clone());
            }
        }
        
        None
    }
    
    pub fn get_content_type(&self) -> String {
        if let Some(ct) = self.get_header("content-type") {
            ct.value.clone()
        } else {
            "application/octet-stream".to_string()
        }
    }
    
    pub fn to_attachment(&self) -> Result<Attachment> {
        let filename = self.get_filename()
            .unwrap_or_else(|| "attachment".to_string());
        
        let content_type = self.get_content_type();
        
        // Check if content is already base64 encoded
        let encoding = self.get_header("content-transfer-encoding")
            .map(|h| h.value.to_lowercase())
            .unwrap_or_else(|| "7bit".to_string());
        
        let content = if encoding == "base64" {
            // Content is already base64 encoded, clean it up
            let content_str = String::from_utf8_lossy(&self.body);
            content_str.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
        } else {
            // Content needs to be base64 encoded
            general_purpose::STANDARD.encode(&self.body)
        };
        
        Ok(Attachment {
            name: filename,
            content,
            content_type,
            cid: None, // TODO: Handle Content-ID if needed
        })
    }
}

pub struct MimeParser {
    max_attachment_size: usize,
    max_attachments: usize,
}

impl MimeParser {
    pub fn new(max_attachment_size: usize, max_attachments: usize) -> Self {
        Self {
            max_attachment_size,
            max_attachments,
        }
    }
    
    pub fn parse_email(&self, email_content: &str) -> Result<(HashMap<String, String>, String, Vec<Attachment>)> {
        let mut headers = HashMap::new();
        let mut body_lines = Vec::new();
        let mut in_headers = true;
        
        // First pass: separate headers from body
        for line in email_content.lines() {
            if in_headers {
                if line.is_empty() {
                    in_headers = false;
                    continue;
                }
                
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_lowercase();
                    let value = line[colon_pos + 1..].trim().to_string();
                    headers.insert(key, value);
                }
            } else {
                body_lines.push(line);
            }
        }
        
        let body_content = body_lines.join("\n");
        
        // Check if this is a multipart message
        if let Some(content_type) = headers.get("content-type") {
            if content_type.starts_with("multipart/") {
                let boundary = self.extract_boundary(content_type)?;
                let (text_body, attachments) = self.parse_multipart(&body_content, &boundary)?;
                return Ok((headers, text_body, attachments));
            }
        }
        
        // Single part message - no attachments
        Ok((headers, body_content, Vec::new()))
    }
    
    fn extract_boundary(&self, content_type: &str) -> Result<String> {
        for part in content_type.split(';') {
            let part = part.trim();
            if part.starts_with("boundary=") {
                let boundary = &part[9..];
                return Ok(boundary.trim_matches('"').to_string());
            }
        }
        Err(anyhow::anyhow!("No boundary found in Content-Type"))
    }
    
    fn parse_multipart(&self, body: &str, boundary: &str) -> Result<(String, Vec<Attachment>)> {
        let boundary_start = format!("--{}", boundary);
        let boundary_end = format!("--{}--", boundary);
        
        let mut parts = Vec::new();
        let mut current_part = Vec::new();
        let mut in_part = false;
        
        for line in body.lines() {
            if line == boundary_start {
                if in_part && !current_part.is_empty() {
                    parts.push(current_part.join("\n"));
                    current_part.clear();
                }
                in_part = true;
            } else if line == boundary_end {
                if in_part && !current_part.is_empty() {
                    parts.push(current_part.join("\n"));
                }
                break;
            } else if in_part {
                current_part.push(line);
            }
        }
        
        let mut text_body = String::new();
        let mut attachments = Vec::new();
        
        for part_content in parts {
            let mime_part = self.parse_mime_part(&part_content)?;
            
            if mime_part.is_attachment() {
                if attachments.len() >= self.max_attachments {
                    warn!("Maximum number of attachments ({}) exceeded, skipping", self.max_attachments);
                    continue;
                }
                
                if mime_part.body.len() > self.max_attachment_size {
                    warn!("Attachment too large ({} bytes), skipping", mime_part.body.len());
                    continue;
                }
                
                match mime_part.to_attachment() {
                    Ok(attachment) => {
                        debug!("Found attachment: {}", attachment.name);
                        attachments.push(attachment);
                    }
                    Err(e) => {
                        warn!("Failed to process attachment: {}", e);
                    }
                }
            } else {
                // This is likely the text body
                let content_type = mime_part.get_content_type();
                if content_type.starts_with("text/") {
                    let body_text = String::from_utf8_lossy(&mime_part.body);
                    if !text_body.is_empty() {
                        text_body.push('\n');
                    }
                    text_body.push_str(&body_text);
                }
            }
        }
        
        Ok((text_body, attachments))
    }
    
    fn parse_mime_part(&self, part_content: &str) -> Result<MimePart> {
        let mut part = MimePart::new();
        let mut body_lines = Vec::new();
        let mut in_headers = true;
        
        for line in part_content.lines() {
            if in_headers {
                if line.is_empty() {
                    in_headers = false;
                    continue;
                }
                
                if let Ok(header) = MimeHeader::parse(line) {
                    part.headers.insert(header.name.clone(), header);
                }
            } else {
                body_lines.push(line);
            }
        }
        
        part.body = body_lines.join("\n").into_bytes();
        Ok(part)
    }
}
