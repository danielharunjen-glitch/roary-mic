use log::debug;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use serde_json::json;

const ELEVENLABS_BASE: &str = "https://api.elevenlabs.io/v1/text-to-speech";

/// Build the full URL for an ElevenLabs TTS request for a given voice.
/// Kept pure so it can be unit-tested without hitting the network.
pub(crate) fn build_elevenlabs_url(voice_id: &str) -> String {
    format!("{}/{}", ELEVENLABS_BASE, voice_id)
}

/// Build the headers for an ElevenLabs TTS request. Returns an error if the
/// supplied API key cannot be encoded as an HTTP header value.
pub(crate) fn build_elevenlabs_headers(api_key: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "xi-api-key",
        HeaderValue::from_str(api_key)
            .map_err(|e| format!("Invalid ElevenLabs API key value: {}", e))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT, HeaderValue::from_static("audio/mpeg"));
    Ok(headers)
}

/// Format a human-readable error message for a failed ElevenLabs request.
pub(crate) fn format_elevenlabs_error(status: reqwest::StatusCode, body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        format!("ElevenLabs request failed with status {}", status)
    } else {
        format!(
            "ElevenLabs request failed with status {}: {}",
            status, trimmed
        )
    }
}

/// Send a text-to-speech request to ElevenLabs and return the raw MP3 bytes.
pub async fn speak_via_elevenlabs(
    api_key: &str,
    voice_id: &str,
    model_id: &str,
    text: &str,
) -> Result<Vec<u8>, String> {
    if api_key.trim().is_empty() {
        return Err("ElevenLabs API key is not set".to_string());
    }
    if voice_id.trim().is_empty() {
        return Err("ElevenLabs voice ID is not set".to_string());
    }
    if text.trim().is_empty() {
        return Err("Nothing to speak".to_string());
    }

    let url = build_elevenlabs_url(voice_id);
    let headers = build_elevenlabs_headers(api_key)?;
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to build ElevenLabs HTTP client: {}", e))?;

    let body = json!({
        "text": text,
        "model_id": model_id,
    });

    debug!(
        "Requesting ElevenLabs TTS (voice={}, model={}, chars={})",
        voice_id,
        model_id,
        text.chars().count()
    );

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("ElevenLabs HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let text_body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unreadable error body>".to_string());
        return Err(format_elevenlabs_error(status, &text_body));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read ElevenLabs response body: {}", e))?;
    Ok(bytes.to_vec())
}

/// Play an MP3 byte buffer on the default output device. Blocks the caller
/// thread until playback finishes.
pub fn play_mp3_blocking(mp3_bytes: Vec<u8>) -> Result<(), String> {
    use std::io::Cursor;

    let stream_handle = rodio::OutputStreamBuilder::from_default_device()
        .map_err(|e| format!("Failed to pick default audio device: {}", e))?
        .open_stream()
        .map_err(|e| format!("Failed to open audio output stream: {}", e))?;
    let mixer = stream_handle.mixer();

    let cursor = Cursor::new(mp3_bytes);
    let sink = rodio::play(mixer, cursor).map_err(|e| format!("Failed to decode MP3: {}", e))?;
    sink.sleep_until_end();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevenlabs_url_contains_voice_id() {
        let url = build_elevenlabs_url("21m00Tcm4TlvDq8ikWAM");
        assert_eq!(
            url,
            "https://api.elevenlabs.io/v1/text-to-speech/21m00Tcm4TlvDq8ikWAM"
        );
    }

    #[test]
    fn elevenlabs_headers_include_api_key_and_json() {
        let headers = build_elevenlabs_headers("sk-test").unwrap();
        assert_eq!(
            headers.get("xi-api-key").and_then(|v| v.to_str().ok()),
            Some("sk-test")
        );
        assert_eq!(
            headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
        assert_eq!(
            headers.get(ACCEPT).and_then(|v| v.to_str().ok()),
            Some("audio/mpeg")
        );
    }

    #[test]
    fn elevenlabs_headers_reject_newline_in_key() {
        let err = build_elevenlabs_headers("bad\nkey").unwrap_err();
        assert!(err.contains("Invalid ElevenLabs API key"));
    }

    #[test]
    fn error_formatter_includes_body_when_present() {
        let msg = format_elevenlabs_error(
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"detail":"Invalid API key"}"#,
        );
        assert!(msg.contains("401"));
        assert!(msg.contains("Invalid API key"));
    }

    #[test]
    fn error_formatter_handles_empty_body() {
        let msg = format_elevenlabs_error(reqwest::StatusCode::BAD_GATEWAY, "   ");
        assert!(msg.contains("502"));
        assert!(!msg.contains(":"));
    }

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tauri::async_runtime::block_on(fut)
    }

    #[test]
    fn speak_rejects_empty_key() {
        let err = block_on(speak_via_elevenlabs("", "v1", "eleven_turbo_v2_5", "hi")).unwrap_err();
        assert!(err.contains("API key"));
    }

    #[test]
    fn speak_rejects_empty_voice() {
        let err = block_on(speak_via_elevenlabs("k", "", "eleven_turbo_v2_5", "hi")).unwrap_err();
        assert!(err.contains("voice"));
    }

    #[test]
    fn speak_rejects_empty_text() {
        let err = block_on(speak_via_elevenlabs("k", "v", "eleven_turbo_v2_5", "   ")).unwrap_err();
        assert!(err.to_lowercase().contains("speak"));
    }
}
