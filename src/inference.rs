#![allow(unused)]

use anyhow::{Result, bail};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

/// Request body for OpenAI-compatible API
#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: Option<u32>,
}

/// Individual message in the chat
#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Response from OpenAI-compatible API
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

/// Choice in the response
#[derive(Debug, Deserialize)]
struct Choice {
    message: CompletionMessage,
}

/// Message in the response
#[derive(Debug, Deserialize)]
struct CompletionMessage {
    content: String,
}

pub struct InferenceService {
    api_url: String,
    api_key: String,
    model: String,
    client: Client,
}

impl InferenceService {
    pub fn new(api_url: &str, api_key: &str, model: &str, client: Client) -> Self {
        Self {
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            client,
        }
    }

    /// Infers with a language model from an OpenAI-compatible API.
    pub async fn infer(&self, prompt: &str) -> Result<String> {
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.1,
            max_tokens: Some(2048),
        };

        let response = self.build_request(request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!("API request failed with status {status}: {error_text}");
        }

        let completion_response: ChatCompletionResponse = response.json().await?;

        if let Some(choice) = completion_response.choices.first() {
            Ok(self.fix_response(&choice.message.content))
        } else {
            bail!("No completion found in API response")
        }
    }

    fn build_request(&self, request: ChatCompletionRequest) -> RequestBuilder {
        self.client
            .post(format!("{}/chat/completions", self.api_url))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
    }

    fn fix_response(&self, text: &str) -> String {
        return text.replace("*", "").replace("—", ", ");
    }
}
