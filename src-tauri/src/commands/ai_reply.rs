use crate::ai_reply::hide_ai_reply_window;
use crate::settings::get_settings;
use crate::tts;
use crate::utils;
use log::{debug, error};
use tauri::AppHandle;

#[tauri::command]
#[specta::specta]
pub async fn ai_reply_paste(app: AppHandle, text: String) -> Result<(), String> {
    // Hop to the main thread — Enigo paste requires it on macOS. Use a
    // oneshot so the paste result is awaited and returned through the command
    // Result; otherwise the window would be hidden with the user believing the
    // paste succeeded while it silently failed.
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let ah = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(utils::paste(text, ah));
    })
    .map_err(|e| format!("Failed to run paste on main thread: {}", e))?;

    match rx.await {
        Ok(Ok(())) => {
            debug!("AI reply text pasted via window");
            hide_ai_reply_window(&app);
            Ok(())
        }
        Ok(Err(e)) => {
            error!("Failed to paste AI reply text: {}", e);
            Err(e)
        }
        Err(e) => Err(format!("Paste result channel closed: {}", e)),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn ai_reply_speak(app: AppHandle, text: String) -> Result<(), String> {
    let settings = get_settings(&app);
    let api_key = settings.elevenlabs_api_key();
    let voice_id = settings.elevenlabs_voice_id.clone();
    let model_id = settings.elevenlabs_model_id.clone();

    // Fetch the MP3 first; only hide the window on success so the user can
    // read the error banner if TTS fails (bad key, 401, network error).
    let mp3 = tts::speak_via_elevenlabs(&api_key, &voice_id, &model_id, &text).await?;

    hide_ai_reply_window(&app);

    // Playback blocks until the audio finishes — offload it so the command
    // resolves immediately and the event loop doesn't hang.
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = tts::play_mp3_blocking(mp3) {
            error!("Failed to play ElevenLabs MP3: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn ai_reply_cancel(app: AppHandle) -> Result<(), String> {
    hide_ai_reply_window(&app);
    Ok(())
}
