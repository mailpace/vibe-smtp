use anyhow::Result;
use minify_html::{minify, Cfg};
use std::panic;
use tracing::{debug, warn};

pub struct HtmlCompressor {
    config: Cfg,
}

impl HtmlCompressor {
    pub fn new() -> Self {
        let config = Cfg {
            do_not_minify_doctype: false,
            ensure_spec_compliant_unquoted_attribute_values: true,
            keep_closing_tags: true, // Keep for email client compatibility
            keep_html_and_head_opening_tags: true, // Keep for email client compatibility
            keep_spaces_between_attributes: false,
            keep_comments: false, // Remove comments
            minify_css: true,
            minify_js: true,
            remove_bangs: false,
            remove_processing_instructions: true,
            ..Cfg::default()
        };

        Self { config }
    }

    /// Compress HTML content if it appears to be valid HTML
    pub fn compress_html(&self, html_content: &str) -> Result<String> {
        // Quick check if content looks like HTML
        if !self.is_html_content(html_content) {
            debug!("Content doesn't appear to be HTML, skipping compression");
            return Ok(html_content.to_string());
        }

        debug!(
            "Compressing HTML content (original size: {} bytes)",
            html_content.len()
        );

        let original_bytes = html_content.as_bytes();

        match panic::catch_unwind(|| minify(original_bytes, &self.config)) {
            Ok(compressed_bytes) => {
                let compressed = String::from_utf8_lossy(&compressed_bytes).to_string();
                let original_size = html_content.len();
                let compressed_size = compressed.len();
                let compression_ratio = if original_size > 0 {
                    ((original_size - compressed_size) as f64 / original_size as f64) * 100.0
                } else {
                    0.0
                };

                debug!(
                    "HTML compression successful: {} -> {} bytes ({:.1}% reduction)",
                    original_size, compressed_size, compression_ratio
                );

                Ok(compressed)
            }
            Err(_) => {
                warn!("HTML compression failed (panic caught), using original content");
                // Return original content if compression fails
                Ok(html_content.to_string())
            }
        }
    }

    /// Simple heuristic to detect if content is HTML
    fn is_html_content(&self, content: &str) -> bool {
        let content_lower = content.trim().to_lowercase();

        // Check for common HTML indicators
        content_lower.contains("<html")
            || content_lower.contains("<!doctype html")
            || content_lower.contains("<head")
            || content_lower.contains("<body")
            || content_lower.contains("<div")
            || content_lower.contains("<p")
            || content_lower.contains("<span")
            || content_lower.contains("<table")
            || (content_lower.contains('<')
                && content_lower.contains('>')
                && (content_lower.contains("<br")
                    || content_lower.contains("<img")
                    || content_lower.contains("<a ")
                    || content_lower.contains("<strong")
                    || content_lower.contains("<em")
                    || content_lower.contains("<h1")
                    || content_lower.contains("<h2")
                    || content_lower.contains("<h3")))
    }
}

impl Default for HtmlCompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_compression_basic() {
        let compressor = HtmlCompressor::new();
        let html = r#"
        <html>
            <head>
                <title>Test Email</title>
            </head>
            <body>
                <h1>Hello World</h1>
                <p>This is a test email with some HTML content.</p>
            </body>
        </html>
        "#;

        let result = compressor.compress_html(html).unwrap();

        // Compressed version should be smaller
        assert!(result.len() < html.len());

        // Should still contain the essential content
        assert!(result.contains("Hello World"));
        assert!(result.contains("This is a test email"));
    }

    #[test]
    fn test_html_compression_with_comments() {
        let compressor = HtmlCompressor::new();
        let html = r#"
        <html>
            <!-- This is a comment that should be removed -->
            <body>
                <p>Content</p>
                <!-- Another comment -->
            </body>
        </html>
        "#;

        let result = compressor.compress_html(html).unwrap();

        // Comments should be removed
        assert!(!result.contains("This is a comment"));
        assert!(!result.contains("Another comment"));

        // Content should remain
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_html_compression_with_whitespace() {
        let compressor = HtmlCompressor::new();
        let html = r#"
        <html>
            <body>
                <p>   Spaced    content   </p>
                <div>
                    
                    <span>  Text  </span>
                    
                </div>
            </body>
        </html>
        "#;

        let result = compressor.compress_html(html).unwrap();

        // Should be significantly smaller due to whitespace removal
        assert!(result.len() < html.len());

        // Content should still be present
        assert!(result.contains("Spaced"));
        assert!(result.contains("content"));
        assert!(result.contains("Text"));
    }

    #[test]
    fn test_is_html_content_positive() {
        let compressor = HtmlCompressor::new();

        assert!(compressor.is_html_content("<html><body>Content</body></html>"));
        assert!(compressor.is_html_content("<!DOCTYPE html><html>"));
        assert!(compressor.is_html_content("<div>Content</div>"));
        assert!(compressor.is_html_content("<p>Simple paragraph</p>"));
        assert!(compressor.is_html_content("Text with <br> tags"));
        assert!(compressor.is_html_content("Link: <a href='#'>click</a>"));
    }

    #[test]
    fn test_is_html_content_negative() {
        let compressor = HtmlCompressor::new();

        assert!(!compressor.is_html_content("Plain text content"));
        assert!(!compressor.is_html_content("Email with some words"));
        assert!(!compressor.is_html_content("Numbers and symbols: 123 @ # $"));
        assert!(!compressor.is_html_content(""));
    }

    #[test]
    fn test_non_html_content_passthrough() {
        let compressor = HtmlCompressor::new();
        let plain_text = "This is just plain text without any HTML tags.";

        let result = compressor.compress_html(plain_text).unwrap();

        // Non-HTML content should pass through unchanged
        assert_eq!(result, plain_text);
    }

    #[test]
    fn test_malformed_html_fallback() {
        let compressor = HtmlCompressor::new();
        let malformed_html = "<html><body><p>Unclosed paragraph<div>Content</body>";

        let result = compressor.compress_html(malformed_html);

        // Should not panic and should return some result
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_html() {
        let compressor = HtmlCompressor::new();
        let empty_html = "";

        let result = compressor.compress_html(empty_html).unwrap();

        // Empty content should remain empty
        assert_eq!(result, "");
    }
}
