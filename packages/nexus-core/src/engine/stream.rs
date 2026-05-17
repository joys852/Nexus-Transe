use futures_util::StreamExt;
use serde::Deserialize;
use crate::Result;

#[derive(Debug, Deserialize)]
pub struct StreamEvent {
    pub event: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Parse SSE lines from a byte stream.
pub async fn read_sse_stream(
    response: reqwest::Response,
    mut on_event: impl FnMut(StreamEvent) -> Result<()> + Send,
) -> Result<()> {
    let mut buf = String::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(map_reqwest)?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buf.find("\n\n") {
            let block = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();
            if let Some(ev) = parse_sse_block(&block) {
                on_event(ev)?;
            }
        }
    }
    Ok(())
}

fn parse_sse_block(block: &str) -> Option<StreamEvent> {
    let mut event_type = "message".to_string();
    let mut data = String::new();
    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("event:") {
            event_type = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("data:") {
            data = rest.trim().to_string();
        }
    }
    if data.is_empty() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&data).unwrap_or(serde_json::json!({ "raw": data }));
    Some(StreamEvent {
        event: event_type,
        data: value,
    })
}

fn map_reqwest(err: reqwest::Error) -> crate::NexusError {
    crate::NexusError::Engine(err.to_string())
}
