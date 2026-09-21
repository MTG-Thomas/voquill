use crate::config::{self, Config, TranscriptionMode};
use crate::{domain_vocabulary, openvino_whisper};
use crate::transcription::TranscriptionService;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

fn common_word_prefix(previous: &str, current: &str) -> String {
    let previous_words: Vec<&str> = previous.split_whitespace().collect();
    let current_words: Vec<&str> = current.split_whitespace().collect();
    let mut stable_words = Vec::new();

    for (previous_word, current_word) in previous_words.iter().zip(current_words.iter()) {
        if previous_word.eq_ignore_ascii_case(current_word) {
            stable_words.push(*current_word);
        } else {
            break;
        }
    }

    stable_words.join(" ")
}

pub fn suffix_after_committed<'a>(text: &'a str, committed: &str) -> Option<&'a str> {
    if committed.trim().is_empty() {
        return Some(text);
    }

    text.strip_prefix(committed)
}

pub fn should_stream_typewriter(config: &Config) -> bool {
    config.streaming_typewriter
        && config.transcription_mode == TranscriptionMode::Local
        && config.output_method == config::OutputMethod::Typewriter
        && config.local_engine == "OpenVINO GenAI"
}

pub fn spawn_streaming_typewriter(
    app_handle: AppHandle,
    config: Arc<Mutex<Config>>,
    mut partial_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    committed_text: Arc<Mutex<String>>,
    language: Option<String>,
    prompt: Option<String>,
    custom_corrections: String,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut previous_partial = String::new();

        while let Some(partial_audio) = partial_rx.recv().await {
            let (model_size, accelerator, typing_speed, hold_duration) = {
                let config_guard = config.lock().unwrap();
                (
                    config_guard.local_model_size.clone(),
                    config_guard.local_accelerator.clone(),
                    config_guard.typing_speed_interval,
                    config_guard.key_press_duration_ms,
                )
            };

            let service = match openvino_whisper::OpenVinoWhisperService::new(
                &model_size,
                &accelerator,
            ) {
                Ok(service) => service,
                Err(error) => {
                    crate::log_info!(
                        "Streaming typewriter skipped partial: failed to initialize OpenVINO service: {}",
                        error
                    );
                    continue;
                }
            };

            let partial_text = match service
                .transcribe(&partial_audio, language.as_deref(), prompt.as_deref())
                .await
            {
                Ok(text) => domain_vocabulary::correct_transcription(&text, &custom_corrections),
                Err(error) => {
                    crate::log_info!("Streaming typewriter partial failed: {}", error);
                    continue;
                }
            };

            let stable_prefix = if previous_partial.is_empty() {
                String::new()
            } else {
                common_word_prefix(&previous_partial, &partial_text)
            };
            previous_partial = partial_text;

            if stable_prefix.is_empty() {
                continue;
            }

            let new_text = {
                let mut committed = committed_text.lock().unwrap();
                let Some(suffix) = suffix_after_committed(&stable_prefix, &committed) else {
                    crate::log_info!(
                        "Streaming typewriter skipped non-monotonic partial: committed='{}', stable='{}'",
                        committed,
                        stable_prefix
                    );
                    continue;
                };

                let suffix = suffix.trim_end().to_string();
                if suffix.is_empty() {
                    continue;
                }

                committed.push_str(&suffix);
                suffix
            };

            crate::log_info!("Streaming typewriter committing partial: '{}'", new_text);
            let state = app_handle.state::<crate::AppState>();
            if let Err(error) = state
                .display_backend
                .type_text_hardware(&app_handle, &new_text, typing_speed, hold_duration)
                .await
            {
                crate::log_info!("STREAMING TYPEWRITER ERROR: {}", error);
            }
        }
    })
}
