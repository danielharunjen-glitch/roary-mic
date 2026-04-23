use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Maximum time we wait for the `claude` CLI to produce output before killing
/// the subprocess. Chosen to cover typical agent loops while still freeing the
/// user if auth/MFA prompts or rate-limit backoff cause the CLI to hang.
const CLAUDE_CLI_TIMEOUT: Duration = Duration::from_secs(60);

/// Return the path to the `claude` CLI if it is on PATH, otherwise None.
///
/// Uses `which` on Unix / `where` on Windows so we match whatever shell
/// resolution the user's environment performs. We deliberately avoid pulling
/// in the `which` crate — a subprocess to the system tool has zero extra
/// dependencies and the cost is trivial (only called on settings mount).
pub fn claude_cli_available() -> Option<PathBuf> {
    #[cfg(target_family = "unix")]
    let lookup_cmd = "which";
    #[cfg(target_family = "windows")]
    let lookup_cmd = "where";

    let output = Command::new(lookup_cmd).arg("claude").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?.trim();
    if first_line.is_empty() {
        return None;
    }
    Some(PathBuf::from(first_line))
}

/// Extract the final assistant text from Claude Code's stream-json output.
///
/// Claude Code emits newline-delimited JSON with several message types. The
/// terminal message is `{"type":"result","subtype":"success","result":"<text>"}`.
/// If that is absent (early termination, failure modes), fall back to
/// concatenating every `{"type":"assistant"}` message's text content blocks,
/// which matches how the CLI itself assembles the final reply.
pub fn extract_reply_from_stream_json(stream: &str) -> Option<String> {
    let mut assistant_chunks: Vec<String> = Vec::new();

    for line in stream.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };

        let msg_type = value.get("type").and_then(Value::as_str).unwrap_or("");

        if msg_type == "result" {
            if let Some(result) = value.get("result").and_then(Value::as_str) {
                return Some(result.to_string());
            }
        }

        if msg_type == "assistant" {
            if let Some(content) = value
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(Value::as_array)
            {
                for block in content {
                    if block.get("type").and_then(Value::as_str) == Some("text") {
                        if let Some(text) = block.get("text").and_then(Value::as_str) {
                            assistant_chunks.push(text.to_string());
                        }
                    }
                }
            }
        }
    }

    if assistant_chunks.is_empty() {
        None
    } else {
        Some(assistant_chunks.join(""))
    }
}

/// Run the local `claude` CLI and return the assistant reply.
///
/// The prompt is fed to Claude Code via stdin (not argv) so arbitrarily long
/// prompts and arbitrary characters work without quoting issues. Images are
/// not supported by the Claude Code CLI today — when `image_png` is provided
/// we note that in the prompt so the model knows it's missing visual context.
pub async fn run_claude_code(
    prompt: &str,
    image_png: Option<&[u8]>,
    system_prompt: Option<&str>,
) -> Result<String, String> {
    // Stash the prompt into `stdin`. The CLI flag is `-p -` but in practice
    // passing `-p` without a value and piping on stdin works too; we use the
    // explicit `-p` + stdin approach.
    let full_prompt = match (system_prompt, image_png.is_some()) {
        (Some(sys), true) => format!(
            "{sys}\n\n[Note: a screenshot was captured but cannot be attached via the Claude Code CLI; respond based on the text below.]\n\n{prompt}"
        ),
        (Some(sys), false) => format!("{sys}\n\n{prompt}"),
        (None, true) => format!(
            "[Note: a screenshot was captured but cannot be attached via the Claude Code CLI; respond based on the text below.]\n\n{prompt}"
        ),
        (None, false) => prompt.to_string(),
    };

    // Use tokio::process so the child is killed when the timeout future drops
    // it (kill_on_drop). std::process + spawn_blocking would leak the process
    // and the blocking-pool thread on timeout because spawn_blocking tasks
    // cannot be cancelled.
    let output = tokio::time::timeout(CLAUDE_CLI_TIMEOUT, async move {
        let mut child = tokio::process::Command::new("claude")
            .arg("-p")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .arg("--dangerously-skip-permissions")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to spawn claude CLI: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(full_prompt.as_bytes())
                .await
                .map_err(|e| format!("Failed to write prompt to claude stdin: {e}"))?;
            // Drop stdin so the CLI sees EOF and starts processing.
            drop(stdin);
        }

        child
            .wait_with_output()
            .await
            .map_err(|e| format!("Failed to wait for claude CLI: {e}"))
    })
    .await
    .map_err(|_| {
        format!(
            "claude CLI timed out after {}s (stuck auth prompt, MFA, or rate-limit backoff?)",
            CLAUDE_CLI_TIMEOUT.as_secs()
        )
    })??;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "claude CLI exited with status {:?}: {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    extract_reply_from_stream_json(&stdout)
        .ok_or_else(|| "claude CLI produced no assistant output".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_reply_prefers_result_message() {
        let stream = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"text","text":"Hello"}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":" world"}]}}
{"type":"result","subtype":"success","result":"Hello world"}
"#;
        assert_eq!(
            extract_reply_from_stream_json(stream),
            Some("Hello world".to_string())
        );
    }

    #[test]
    fn extract_reply_concatenates_assistant_text_when_no_result() {
        let stream = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"text","text":"Part 1 "}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":"Part 2"}]}}
"#;
        assert_eq!(
            extract_reply_from_stream_json(stream),
            Some("Part 1 Part 2".to_string())
        );
    }

    #[test]
    fn extract_reply_skips_non_text_content_blocks() {
        let stream = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{}},{"type":"text","text":"Done"}]}}"#;
        assert_eq!(
            extract_reply_from_stream_json(stream),
            Some("Done".to_string())
        );
    }

    #[test]
    fn extract_reply_returns_none_on_empty_stream() {
        assert_eq!(extract_reply_from_stream_json(""), None);
    }

    #[test]
    fn extract_reply_ignores_malformed_lines() {
        let stream = r#"not json at all
{"type":"result","subtype":"success","result":"ok"}
more garbage"#;
        assert_eq!(
            extract_reply_from_stream_json(stream),
            Some("ok".to_string())
        );
    }

    #[test]
    fn extract_reply_returns_none_when_only_non_assistant_messages() {
        let stream = r#"{"type":"system","subtype":"init"}
{"type":"user","message":{"content":"hi"}}
"#;
        assert_eq!(extract_reply_from_stream_json(stream), None);
    }
}
