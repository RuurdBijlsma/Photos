use async_stream::try_stream;
use base64::{Engine as _, engine::general_purpose};
use bon::bon;
use futures_util::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::io::StreamReader;

#[derive(Error, Debug)]
pub enum LlamaError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("API error (status {status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },
}

pub type LlamaResult<T> = Result<T, LlamaError>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<MessagePart>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MessagePart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_object")]
    JsonObject {
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: Option<serde_json::Value>,
    },
}

#[derive(Serialize, Debug)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    temperature: f32,
    top_p: f32,
    repetition_penalty: f32,
    presence_penalty: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Deserialize)]
struct ChatChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Deserialize)]
struct ChunkChoice {
    delta: ChunkDelta,
}

#[derive(Deserialize)]
struct ChunkDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
pub struct ChatFullResponse {
    pub choices: Vec<FullChoice>,
}

#[derive(Deserialize)]
pub struct FullChoice {
    pub message: FullMessage,
}

#[derive(Deserialize)]
pub struct FullMessage {
    pub content: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ChatEvent {
    Content(String),
    Reasoning(String),
}

#[derive(Clone)]
pub struct LlamaConfig {
    pub temperature: f32,
    pub top_p: f32,
    pub repetition_penalty: f32,
    pub presence_penalty: f32,
}

#[derive(Clone)]
pub struct LlamaClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
    config: LlamaConfig,
}

#[bon]
impl LlamaClient {
    #[builder(start_fn = with_base_url)]
    #[must_use]
    pub fn new(
        #[builder(start_fn)] base_url: &str,
        model: Option<String>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        repetition_penalty: Option<f32>,
        presence_penalty: Option<f32>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.to_string(),
            model: model.unwrap_or_default(),
            config: LlamaConfig {
                temperature: temperature.unwrap_or(0.7),
                top_p: top_p.unwrap_or(0.8),
                repetition_penalty: repetition_penalty.unwrap_or(1.0),
                presence_penalty: presence_penalty.unwrap_or(1.5),
            },
        }
    }

    pub async fn prepare_messages(&self, prompt: &str, images: &[&Path]) -> LlamaResult<Message> {
        let mut parts = vec![MessagePart::Text {
            text: prompt.to_string(),
        }];
        for path in images {
            let bytes = fs::read(path).await?;
            let mime_type = infer::get(&bytes).map_or("image/jpeg", |kind| kind.mime_type());
            let b64 = general_purpose::STANDARD.encode(&bytes);
            parts.push(MessagePart::ImageUrl {
                image_url: ImageUrl {
                    url: format!("data:{mime_type};base64,{b64}"),
                },
            });
        }
        Ok(Message {
            role: "user".to_string(),
            content: MessageContent::Parts(parts),
        })
    }

    #[builder]
    pub async fn chat(
        &self,
        #[builder(start_fn)] prompt: &str,
        images: Option<&[&Path]>,
        schema: Option<serde_json::Value>,
    ) -> LlamaResult<String> {
        let msg = self
            .prepare_messages(prompt, images.unwrap_or_default())
            .await?;

        // Pass schema to the call method
        let req_body = self.build_request(vec![msg], false, schema);
        let url = format!("{}/v1/chat/completions", self.base_url);
        let response = self.http.post(url).json(&req_body).send().await?;

        if !response.status().is_success() {
            return Err(LlamaError::Api {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            });
        }
        let full: ChatFullResponse = response.json().await?;
        Ok(full
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    #[builder]
    pub async fn chat_stream(
        &self,
        #[builder(start_fn)] prompt: &str,
        images: Option<&[&Path]>,
    ) -> LlamaResult<Pin<Box<dyn Stream<Item = LlamaResult<ChatEvent>> + Send>>> {
        let msg = self
            .prepare_messages(prompt, images.unwrap_or_default())
            .await?;
        self.call_stream(vec![msg]).await
    }

    pub async fn call(&self, messages: Vec<Message>) -> LlamaResult<String> {
        let req_body = self.build_request(messages, false, None);
        let url = format!("{}/v1/chat/completions", self.base_url);
        let response = self.http.post(url).json(&req_body).send().await?;
        if !response.status().is_success() {
            return Err(LlamaError::Api {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            });
        }
        let full: ChatFullResponse = response.json().await?;
        Ok(full
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    pub async fn call_stream(
        &self,
        messages: Vec<Message>,
    ) -> LlamaResult<Pin<Box<dyn Stream<Item = LlamaResult<ChatEvent>> + Send>>> {
        let req_body = self.build_request(messages, true, None);
        let url = format!("{}/v1/chat/completions", self.base_url);
        let response = self.http.post(url).json(&req_body).send().await?;
        if !response.status().is_success() {
            return Err(LlamaError::Api {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            });
        }
        let stream_bytes = response.bytes_stream().map_err(std::io::Error::other);
        let reader = StreamReader::new(stream_bytes);
        let mut lines = BufReader::new(reader).lines();
        Ok(Box::pin(try_stream! {
            while let Some(line) = lines.next_line().await.map_err(LlamaError::Io)? {
                let line = line.trim();
                if line.is_empty() || line == "data: [DONE]" { continue; }
                if let Some(data) = line.strip_prefix("data: ") {
                    let chunk = serde_json::from_str::<ChatChunk>(data).map_err(LlamaError::Json)?;
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(r) = &choice.delta.reasoning_content {
                            yield ChatEvent::Reasoning(r.clone());
                        }
                        if let Some(c) = &choice.delta.content {
                            yield ChatEvent::Content(c.clone());
                        }
                    }
                }
            }
        }))
    }

    fn build_request(
        &self,
        messages: Vec<Message>,
        stream: bool,
        schema: Option<serde_json::Value>,
    ) -> ChatRequest {
        let response_format = schema.map(|s| ResponseFormat::JsonObject { schema: Some(s) });

        ChatRequest {
            model: self.model.clone(),
            messages,
            stream,
            top_p: self.config.top_p,
            temperature: self.config.temperature,
            repetition_penalty: self.config.repetition_penalty,
            presence_penalty: self.config.presence_penalty,
            response_format,
        }
    }
}

pub struct ChatSession {
    client: LlamaClient,
    pub messages: Vec<Message>,
}

#[bon]
impl ChatSession {
    #[must_use]
    pub const fn new(client: LlamaClient) -> Self {
        Self {
            client,
            messages: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.messages.clear();
    }

    #[builder]
    pub async fn chat(
        &mut self,
        #[builder(start_fn)] prompt: &str,
        images: Option<&[&Path]>,
    ) -> LlamaResult<String> {
        let msg = self
            .client
            .prepare_messages(prompt, images.unwrap_or_default())
            .await?;
        self.messages.push(msg);
        let response = self.client.call(self.messages.clone()).await?;
        self.messages.push(Message {
            role: "assistant".to_string(),
            content: MessageContent::Text(response.clone()),
        });
        Ok(response)
    }

    #[builder]
    pub async fn chat_stream<'a>(
        &'a mut self,
        #[builder(start_fn)] prompt: &str,
        images: Option<&[&Path]>,
    ) -> LlamaResult<ChatResponseStream<'a>> {
        let msg = self
            .client
            .prepare_messages(prompt, images.unwrap_or_default())
            .await?;
        self.messages.push(msg);
        let inner = self.client.call_stream(self.messages.clone()).await?;
        Ok(ChatResponseStream {
            inner,
            session: self,
            accumulated_content: String::new(),
        })
    }
}

pub struct ChatResponseStream<'a> {
    inner: Pin<Box<dyn Stream<Item = LlamaResult<ChatEvent>> + Send>>,
    session: &'a mut ChatSession,
    accumulated_content: String,
}

impl Stream for ChatResponseStream<'_> {
    type Item = LlamaResult<ChatEvent>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let result = self.inner.poll_next_unpin(cx);
        match &result {
            Poll::Ready(Some(Ok(ChatEvent::Content(c)))) => {
                self.accumulated_content.push_str(c);
            }
            Poll::Ready(None) => {
                let content = std::mem::take(&mut self.accumulated_content);
                if !content.is_empty() {
                    self.session.messages.push(Message {
                        role: "assistant".to_string(),
                        content: MessageContent::Text(content),
                    });
                }
            }
            _ => {}
        }
        result
    }
}
