use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[derive(Serialize, Debug)]
pub struct MailPacePayload {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub htmlbody: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub textbody: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replyto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_unsubscribe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Debug)]
pub struct Attachment {
    pub name: String,
    pub content: String,
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
}

#[derive(Deserialize)]
pub struct MailPaceResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub errors: Option<Vec<String>>,
}

pub struct MailPaceClient {
    client: Client,
    endpoint: String,
}

impl MailPaceClient {
    pub fn new(client: Client, endpoint: String) -> Self {
        Self { client, endpoint }
    }

    pub async fn send_email(&self, payload: &MailPacePayload, token: &str) -> Result<()> {
        debug!("Sending payload to MailPace: {:?}", payload);
        
        let response = self.client
            .post(&self.endpoint)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("MailPace-Server-Token", token)
            .json(payload)
            .send()
            .await
            .context("Failed to send request to MailPace API")?;
        
        if response.status().is_success() {
            let mailpace_response: MailPaceResponse = response.json().await
                .context("Failed to parse MailPace response")?;
            info!("Email sent successfully, ID: {:?}", mailpace_response.id);
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            Err(anyhow::anyhow!("MailPace API error ({}): {}", status, error_text))
        }
    }
}
