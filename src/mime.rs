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

impl Default for MimePart {
    fn default() -> Self {
        Self::new()
    }
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
        let filename = self
            .get_filename()
            .unwrap_or_else(|| "attachment".to_string());

        let content_type = self.get_content_type();

        // Check if content is already base64 encoded
        let encoding = self
            .get_header("content-transfer-encoding")
            .map(|h| h.value.to_lowercase())
            .unwrap_or_else(|| "7bit".to_string());

        let content = if encoding == "base64" {
            // Content is already base64 encoded, clean it up
            let content_str = String::from_utf8_lossy(&self.body);
            content_str
                .chars()
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

    pub fn parse_email(
        &self,
        email_content: &str,
    ) -> Result<(HashMap<String, String>, String, Vec<Attachment>)> {
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
            if part.to_lowercase().starts_with("boundary") {
                if let Some(eq_pos) = part.find('=') {
                    let boundary = part[eq_pos + 1..].trim();
                    return Ok(boundary.trim_matches('"').to_string());
                }
            }
        }
        Err(anyhow::anyhow!("No boundary found in Content-Type"))
    }

    fn parse_multipart(&self, body: &str, boundary: &str) -> Result<(String, Vec<Attachment>)> {
        let boundary_start = format!("--{boundary}");
        let boundary_end = format!("--{boundary}--");

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
                    warn!(
                        "Maximum number of attachments ({}) exceeded, skipping",
                        self.max_attachments
                    );
                    continue;
                }

                if mime_part.body.len() > self.max_attachment_size {
                    warn!(
                        "Attachment too large ({} bytes), skipping",
                        mime_part.body.len()
                    );
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
        let mut found_header = false;

        for line in part_content.lines() {
            if in_headers {
                if line.is_empty() {
                    in_headers = false;
                    continue;
                }

                if line.contains(':') {
                    if let Ok(header) = MimeHeader::parse(line) {
                        part.headers.insert(header.name.clone(), header);
                        found_header = true;
                    }
                } else if !found_header {
                    // If we haven't found any headers yet and this line doesn't contain ':',
                    // treat this as body content (no headers at all)
                    in_headers = false;
                    body_lines.push(line);
                }
            } else {
                body_lines.push(line);
            }
        }

        part.body = body_lines.join("\n").into_bytes();
        Ok(part)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_header_parse_simple() {
        let header = MimeHeader::parse("Content-Type: text/plain").unwrap();
        assert_eq!(header.name, "content-type");
        assert_eq!(header.value, "text/plain");
        assert_eq!(header.params.len(), 0);
    }

    #[test]
    fn test_mime_header_parse_with_params() {
        let header =
            MimeHeader::parse("Content-Type: text/plain; charset=utf-8; boundary=\"test123\"")
                .unwrap();
        assert_eq!(header.name, "content-type");
        assert_eq!(header.value, "text/plain");
        assert_eq!(header.params.get("charset"), Some(&"utf-8".to_string()));
        assert_eq!(header.params.get("boundary"), Some(&"test123".to_string()));
    }

    #[test]
    fn test_mime_header_parse_no_value() {
        let header = MimeHeader::parse("Content-Type:").unwrap();
        assert_eq!(header.name, "content-type");
        assert_eq!(header.value, "");
        assert_eq!(header.params.len(), 0);
    }

    #[test]
    fn test_mime_header_parse_no_colon() {
        let header = MimeHeader::parse("InvalidHeader").unwrap();
        assert_eq!(header.name, "invalidheader");
        assert_eq!(header.value, "");
    }

    #[test]
    fn test_mime_header_get_param() {
        let header = MimeHeader::parse("Content-Type: text/plain; charset=utf-8").unwrap();
        assert_eq!(header.get_param("charset"), Some(&"utf-8".to_string()));
        assert_eq!(header.get_param("nonexistent"), None);
    }

    #[test]
    fn test_mime_part_new() {
        let part = MimePart::new();
        assert_eq!(part.headers.len(), 0);
        assert_eq!(part.body.len(), 0);
    }

    #[test]
    fn test_mime_part_default() {
        let part = MimePart::default();
        assert_eq!(part.headers.len(), 0);
        assert_eq!(part.body.len(), 0);
    }

    #[test]
    fn test_mime_part_get_header() {
        let mut part = MimePart::new();
        let header = MimeHeader::parse("Content-Type: text/plain").unwrap();
        part.headers.insert("content-type".to_string(), header);

        assert!(part.get_header("Content-Type").is_some());
        assert!(part.get_header("content-type").is_some());
        assert!(part.get_header("nonexistent").is_none());
    }

    #[test]
    fn test_mime_part_is_attachment_true() {
        let mut part = MimePart::new();
        let header =
            MimeHeader::parse("Content-Disposition: attachment; filename=\"test.txt\"").unwrap();
        part.headers
            .insert("content-disposition".to_string(), header);

        assert!(part.is_attachment());
    }

    #[test]
    fn test_mime_part_is_attachment_false() {
        let mut part = MimePart::new();
        let header = MimeHeader::parse("Content-Disposition: inline").unwrap();
        part.headers
            .insert("content-disposition".to_string(), header);

        assert!(!part.is_attachment());
    }

    #[test]
    fn test_mime_part_is_attachment_no_header() {
        let part = MimePart::new();
        assert!(!part.is_attachment());
    }

    #[test]
    fn test_mime_part_get_filename_from_disposition() {
        let mut part = MimePart::new();
        let header =
            MimeHeader::parse("Content-Disposition: attachment; filename=\"test.txt\"").unwrap();
        part.headers
            .insert("content-disposition".to_string(), header);

        assert_eq!(part.get_filename(), Some("test.txt".to_string()));
    }

    #[test]
    fn test_mime_part_get_filename_from_content_type() {
        let mut part = MimePart::new();
        let header = MimeHeader::parse("Content-Type: text/plain; name=\"test.txt\"").unwrap();
        part.headers.insert("content-type".to_string(), header);

        assert_eq!(part.get_filename(), Some("test.txt".to_string()));
    }

    #[test]
    fn test_mime_part_get_filename_none() {
        let part = MimePart::new();
        assert_eq!(part.get_filename(), None);
    }

    #[test]
    fn test_mime_part_get_content_type_default() {
        let part = MimePart::new();
        assert_eq!(part.get_content_type(), "application/octet-stream");
    }

    #[test]
    fn test_mime_part_get_content_type_custom() {
        let mut part = MimePart::new();
        let header = MimeHeader::parse("Content-Type: text/plain").unwrap();
        part.headers.insert("content-type".to_string(), header);

        assert_eq!(part.get_content_type(), "text/plain");
    }

    #[test]
    fn test_mime_part_to_attachment_base64() {
        let mut part = MimePart::new();

        let disposition_header =
            MimeHeader::parse("Content-Disposition: attachment; filename=\"test.txt\"").unwrap();
        part.headers
            .insert("content-disposition".to_string(), disposition_header);

        let content_type_header = MimeHeader::parse("Content-Type: text/plain").unwrap();
        part.headers
            .insert("content-type".to_string(), content_type_header);

        let encoding_header = MimeHeader::parse("Content-Transfer-Encoding: base64").unwrap();
        part.headers
            .insert("content-transfer-encoding".to_string(), encoding_header);

        part.body = "SGVsbG8gV29ybGQ=".as_bytes().to_vec(); // "Hello World" in base64

        let attachment = part.to_attachment().unwrap();
        assert_eq!(attachment.name, "test.txt");
        assert_eq!(attachment.content_type, "text/plain");
        assert_eq!(attachment.content, "SGVsbG8gV29ybGQ=");
        assert_eq!(attachment.cid, None);
    }

    #[test]
    fn test_mime_part_to_attachment_plain() {
        let mut part = MimePart::new();

        let disposition_header =
            MimeHeader::parse("Content-Disposition: attachment; filename=\"test.txt\"").unwrap();
        part.headers
            .insert("content-disposition".to_string(), disposition_header);

        let content_type_header = MimeHeader::parse("Content-Type: text/plain").unwrap();
        part.headers
            .insert("content-type".to_string(), content_type_header);

        part.body = "Hello World".as_bytes().to_vec();

        let attachment = part.to_attachment().unwrap();
        assert_eq!(attachment.name, "test.txt");
        assert_eq!(attachment.content_type, "text/plain");
        assert_eq!(
            attachment.content,
            general_purpose::STANDARD.encode("Hello World")
        );
        assert_eq!(attachment.cid, None);
    }

    #[test]
    fn test_mime_part_to_attachment_no_filename() {
        let mut part = MimePart::new();

        let content_type_header = MimeHeader::parse("Content-Type: text/plain").unwrap();
        part.headers
            .insert("content-type".to_string(), content_type_header);

        part.body = "Hello World".as_bytes().to_vec();

        let attachment = part.to_attachment().unwrap();
        assert_eq!(attachment.name, "attachment");
        assert_eq!(attachment.content_type, "text/plain");
    }

    #[test]
    fn test_mime_parser_new() {
        let parser = MimeParser::new(1024, 10);
        assert_eq!(parser.max_attachment_size, 1024);
        assert_eq!(parser.max_attachments, 10);
    }

    #[test]
    fn test_mime_parser_parse_simple_email() {
        let parser = MimeParser::new(1024, 10);
        let email = "From: test@example.com\nTo: user@example.com\nSubject: Test\n\nHello World";

        let (headers, body, attachments) = parser.parse_email(email).unwrap();

        assert_eq!(headers.get("from"), Some(&"test@example.com".to_string()));
        assert_eq!(headers.get("to"), Some(&"user@example.com".to_string()));
        assert_eq!(headers.get("subject"), Some(&"Test".to_string()));
        assert_eq!(body, "Hello World");
        assert_eq!(attachments.len(), 0);
    }

    #[test]
    fn test_mime_parser_extract_boundary() {
        let parser = MimeParser::new(1024, 10);

        let content_type = "multipart/mixed; boundary=boundary123";
        assert_eq!(
            parser.extract_boundary(content_type).unwrap(),
            "boundary123"
        );

        let content_type_quoted = "multipart/mixed; boundary=\"boundary123\"";
        assert_eq!(
            parser.extract_boundary(content_type_quoted).unwrap(),
            "boundary123"
        );
    }

    #[test]
    fn test_mime_parser_extract_boundary_error() {
        let parser = MimeParser::new(1024, 10);
        let content_type = "multipart/mixed";
        assert!(parser.extract_boundary(content_type).is_err());
    }

    #[test]
    fn test_mime_parser_parse_multipart_email() {
        let parser = MimeParser::new(1024, 10);
        let email = r#"From: test@example.com
To: user@example.com
Subject: Test with attachment
Content-Type: multipart/mixed; boundary=boundary123

--boundary123
Content-Type: text/plain

Hello World

--boundary123
Content-Type: text/plain
Content-Disposition: attachment; filename="test.txt"

File content here
--boundary123--"#;

        let (headers, body, attachments) = parser.parse_email(email).unwrap();

        assert_eq!(headers.get("from"), Some(&"test@example.com".to_string()));
        assert_eq!(body, "Hello World");
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].name, "test.txt");
    }

    #[test]
    fn test_mime_parser_parse_multipart_multiple_text_parts() {
        let parser = MimeParser::new(1024, 10);
        let email = r#"Content-Type: multipart/mixed; boundary=boundary123

--boundary123
Content-Type: text/plain

First part

--boundary123
Content-Type: text/html

Second part
--boundary123--"#;

        let (_, body, attachments) = parser.parse_email(email).unwrap();

        assert!(body.contains("First part"));
        assert!(body.contains("Second part"));
        assert_eq!(attachments.len(), 0);
    }

    #[test]
    fn test_mime_parser_attachment_size_limit() {
        let parser = MimeParser::new(5, 10); // Very small size limit
        let email = r#"Content-Type: multipart/mixed; boundary=boundary123

--boundary123
Content-Type: text/plain
Content-Disposition: attachment; filename="large.txt"

This content is too large for the limit
--boundary123--"#;

        let (_, _, attachments) = parser.parse_email(email).unwrap();
        assert_eq!(attachments.len(), 0); // Should be skipped due to size
    }

    #[test]
    fn test_mime_parser_attachment_count_limit() {
        let parser = MimeParser::new(1024, 1); // Only 1 attachment allowed
        let email = r#"Content-Type: multipart/mixed; boundary=boundary123

--boundary123
Content-Type: text/plain
Content-Disposition: attachment; filename="file1.txt"

File 1

--boundary123
Content-Type: text/plain
Content-Disposition: attachment; filename="file2.txt"

File 2
--boundary123--"#;

        let (_, _, attachments) = parser.parse_email(email).unwrap();
        assert_eq!(attachments.len(), 1); // Only first attachment should be included
    }

    #[test]
    fn test_mime_parser_parse_mime_part() {
        let parser = MimeParser::new(1024, 10);
        let part_content = r#"Content-Type: text/plain; charset=utf-8
Content-Disposition: attachment; filename="test.txt"

This is the file content"#;

        let part = parser.parse_mime_part(part_content).unwrap();

        assert!(part.get_header("content-type").is_some());
        assert!(part.get_header("content-disposition").is_some());
        assert_eq!(
            String::from_utf8_lossy(&part.body),
            "This is the file content"
        );
    }

    #[test]
    fn test_mime_parser_parse_mime_part_no_headers() {
        let parser = MimeParser::new(1024, 10);
        let part_content = "Just content, no headers";

        let part = parser.parse_mime_part(part_content).unwrap();

        assert_eq!(part.headers.len(), 0);
        assert_eq!(
            String::from_utf8_lossy(&part.body),
            "Just content, no headers"
        );
    }

    #[test]
    fn test_mime_parser_parse_mime_part_empty_line_separator() {
        let parser = MimeParser::new(1024, 10);
        let part_content = r#"Content-Type: text/plain

Body content after empty line"#;

        let part = parser.parse_mime_part(part_content).unwrap();

        assert!(part.get_header("content-type").is_some());
        assert_eq!(
            String::from_utf8_lossy(&part.body),
            "Body content after empty line"
        );
    }

    #[test]
    fn test_mime_header_parse_with_whitespace() {
        let header =
            MimeHeader::parse("  Content-Type  :  text/plain  ; charset = utf-8  ").unwrap();
        assert_eq!(header.name, "content-type");
        assert_eq!(header.value, "text/plain");
        assert_eq!(header.params.get("charset"), Some(&"utf-8".to_string()));
    }

    #[test]
    fn test_mime_part_base64_content_with_whitespace() {
        let mut part = MimePart::new();

        let encoding_header = MimeHeader::parse("Content-Transfer-Encoding: base64").unwrap();
        part.headers
            .insert("content-transfer-encoding".to_string(), encoding_header);

        // Base64 content with whitespace (newlines, spaces)
        part.body = "SGVs\nbG8g\r\n V29y\nbGQ=".as_bytes().to_vec();

        let attachment = part.to_attachment().unwrap();
        // Should clean up whitespace
        assert_eq!(attachment.content, "SGVsbG8gV29ybGQ=");
    }

    #[test]
    fn test_mime_parser_boundary_variations() {
        let parser = MimeParser::new(1024, 10);

        // Test with different boundary styles
        let boundary_tests = vec![
            "multipart/mixed; boundary=simple",
            "multipart/mixed; boundary=\"quoted\"",
            "multipart/mixed; boundary = spaced",
            "multipart/mixed; charset=utf-8; boundary=after_other_param",
        ];

        for content_type in boundary_tests {
            let result = parser.extract_boundary(content_type);
            assert!(
                result.is_ok(),
                "Failed to parse boundary from: {content_type}",
            );
        }
    }
}
