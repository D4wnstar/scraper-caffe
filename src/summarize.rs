use anyhow::{Result, bail};
use reqwest::Client;
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
#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

/// Choice in the response
#[derive(Deserialize)]
struct Choice {
    message: CompletionMessage,
}

/// Message in the response
#[derive(Deserialize)]
struct CompletionMessage {
    content: String,
}

/// Summarize text to a single sentence using an OpenAI-compatible API
pub async fn summarize_text(
    api_url: &str,
    api_key: &str,
    text: &str,
    prompt: &str,
    model: &str,
    client: &Client,
) -> Result<String> {
    let request = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: format!("{prompt}\n\n{text}"),
        }],
        temperature: 0.2,
        max_tokens: Some(100),
    };

    let response = client
        .post(format!("{api_url}/chat/completions"))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        bail!("API request failed with status {status}: {error_text}");
    }

    let completion_response: ChatCompletionResponse = response.json().await?;

    if let Some(choice) = completion_response.choices.first() {
        if let Some(summary) = choice.message.content.trim().split('.').next() {
            Ok(format!("{}.", summary))
        } else {
            Ok(choice.message.content.trim().to_string())
        }
    } else {
        bail!("No summary found in API response")
    }
}
