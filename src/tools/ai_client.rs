//! Shared Anthropic API client for UHM
//!
//! Provides a reusable HTTP client for calling Claude models (Sonnet, Haiku).

use serde_json::Value;

/// Content block for Anthropic messages API
#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text(String),
    Image { base64: String, media_type: String },
}

/// Shared Anthropic API client
pub struct AnthropicClient {
    client: reqwest::blocking::Client,
    api_key: String,
}

impl AnthropicClient {
    /// Create a new client, reading ANTHROPIC_API_KEY from environment.
    /// Returns None if the key is not set.
    pub fn new() -> Option<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;
        Some(Self { client, api_key })
    }

    /// Call Claude Sonnet with text and/or image content
    pub fn call_sonnet(
        &self,
        system: &str,
        content: &[ContentBlock],
        max_tokens: u32,
    ) -> Result<String, String> {
        self.call_model("claude-sonnet-4-5-20250514", system, content, max_tokens)
    }

    /// Call Claude Haiku with text content
    pub fn call_haiku(
        &self,
        system: &str,
        content: &[ContentBlock],
        max_tokens: u32,
    ) -> Result<String, String> {
        self.call_model("claude-haiku-4-5-20251001", system, content, max_tokens)
    }

    /// Internal: call a specific model
    fn call_model(
        &self,
        model: &str,
        system: &str,
        content: &[ContentBlock],
        max_tokens: u32,
    ) -> Result<String, String> {
        let content_blocks: Vec<Value> = content
            .iter()
            .map(|block| match block {
                ContentBlock::Text(text) => serde_json::json!({
                    "type": "text",
                    "text": text
                }),
                ContentBlock::Image { base64, media_type } => serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type,
                        "data": base64
                    }
                }),
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "system": system,
            "messages": [{
                "role": "user",
                "content": content_blocks
            }]
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        let body: Value = response
            .json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body["content"][0]["text"]
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| "No text in API response".to_string())
    }
}
