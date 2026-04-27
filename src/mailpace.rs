use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
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
    #[allow(dead_code)]
    pub status: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub errors: Option<Vec<String>>,
}

pub struct MailPaceClient {
    client: Client,
    endpoint: String,
    retries: usize,
    retry_backoff: Duration,
}

impl MailPaceClient {
    pub fn new(client: Client, endpoint: String, retries: usize, retry_backoff: Duration) -> Self {
        Self {
            client,
            endpoint,
            retries,
            retry_backoff,
        }
    }

    pub async fn send_email(&self, payload: &MailPacePayload, token: &str) -> Result<()> {
        debug!("Sending payload to MailPace: {:?}", payload);
        for attempt in 0..=self.retries {
            match self
                .client
                .post(&self.endpoint)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("MailPace-Server-Token", token)
                .json(payload)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    let mailpace_response: MailPaceResponse = response
                        .json()
                        .await
                        .context("Failed to parse MailPace response")?;
                    info!("Email sent successfully, ID: {:?}", mailpace_response.id);
                    return Ok(());
                }
                Ok(response) => {
                    let status = response.status();
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());

                    if attempt < self.retries && (status.is_server_error() || status.as_u16() == 429)
                    {
                        let delay = self.retry_backoff.saturating_mul(2_u32.pow(attempt as u32));
                        sleep(delay).await;
                        continue;
                    }

                    return Err(anyhow::anyhow!(
                        "MailPace API error ({}): {}",
                        status,
                        error_text
                    ));
                }
                Err(err) => {
                    if attempt < self.retries && (err.is_timeout() || err.is_connect()) {
                        let delay = self.retry_backoff.saturating_mul(2_u32.pow(attempt as u32));
                        sleep(delay).await;
                        continue;
                    }

                    return Err(err).context("Failed to send request to MailPace API");
                }
            }
        }

        Err(anyhow::anyhow!("MailPace API retry loop exited unexpectedly"))
    }
}
