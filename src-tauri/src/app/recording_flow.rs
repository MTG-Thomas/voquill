use crate::config::{self, Config, TranscriptionMode};
use crate::transcription::TranscriptionService;
use crate::{audio, history, local_whisper, openvino_whisper, transcription, typing};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

fn validate_audio_duration(
    audio_data: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if audio_data.len() < 44 {
        return Err("Audio file too small".into());
    }
    let sample_rate = u32::from_le_bytes([
        audio_data[24],
        audio_data[25],
        audio_data[26],
        audio_data[27],
    ]);
    let channels = u16::from_le_bytes([audio_data[22], audio_data[23]]);
    let bits_per_sample = u16::from_le_bytes([audio_data[34], audio_data[35]]);

    let mut data_size = 0u32;
    let mut pos = 36;
    while pos + 8 <= audio_data.len() {
        let chunk_id = &audio_data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            audio_data[pos + 4],
            audio_data[pos + 5],
            audio_data[pos + 6],
            audio_data[pos + 7],
        ]);
        if chunk_id == b"data" {
            data_size = chunk_size;
            break;
        }
        pos += 8 + chunk_size as usize;
        if chunk_size % 2 == 1 {
            pos += 1;
        }
    }

    if data_size == 0 {
        return Err("No data chunk".into());
    }
    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;
    let duration_seconds = data_size as f64 / bytes_per_second as f64;

    crate::log_info!("Audio duration: {:.3}s", duration_seconds);
    if duration_seconds < 0.1 {
        return Err("Audio too short".into());
    }
    Ok(())
}

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

fn suffix_after_committed<'a>(text: &'a str, committed: &str) -> Option<&'a str> {
    if committed.trim().is_empty() {
        return Some(text);
    }

    text.strip_prefix(committed)
}

fn should_stream_typewriter(config: &Config) -> bool {
    config.streaming_typewriter
        && config.transcription_mode == TranscriptionMode::Local
        && config.output_method == config::OutputMethod::Typewriter
        && config.local_engine == "OpenVINO GenAI"
}

fn spawn_streaming_typewriter(
    app_handle: AppHandle,
    config: Arc<Mutex<Config>>,
    mut partial_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    committed_text: Arc<Mutex<String>>,
    language: Option<String>,
    prompt: Option<String>,
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

            let service =
                match openvino_whisper::OpenVinoWhisperService::new(&model_size, &accelerator) {
                    Ok(service) => service,
                    Err(error) => {
                        crate::log_info!(
                            "⚠️ Streaming typewriter skipped partial: failed to initialize OpenVINO service: {}",
                            error
                        );
                        continue;
                    }
                };

            let partial_text = match service
                .transcribe(&partial_audio, language.as_deref(), prompt.as_deref())
                .await
            {
                Ok(text) => text,
                Err(error) => {
                    crate::log_info!("⚠️ Streaming typewriter partial failed: {}", error);
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
                        "⚠️ Streaming typewriter skipped non-monotonic partial: committed='{}', stable='{}'",
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

            crate::log_info!("⌨️ Streaming typewriter committing partial: '{}'", new_text);
            let state = app_handle.state::<crate::AppState>();
            if let Err(error) = state
                .display_backend
                .type_text_hardware(&app_handle, &new_text, typing_speed, hold_duration)
                .await
            {
                crate::log_info!("❌ STREAMING TYPEWRITER ERROR: {}", error);
            }
        }
    })
}

pub async fn record_and_transcribe(
    config: Arc<Mutex<Config>>,
    is_recording: Arc<Mutex<bool>>,
    app_handle: AppHandle,
    audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reset_status_on_exit = || async {
        crate::app::status::emit_status_to_frontend("Ready").await;
    };

    let (
        language_choice,
        streaming_enabled,
        streaming_output_method,
    ) = {
        let config_guard = config.lock().unwrap();
        (
            config_guard.language.clone(),
            should_stream_typewriter(&config_guard),
            config_guard.output_method.clone(),
        )
    };

    let (lang_code, prompt_hint) = match language_choice.as_str() {
        "auto" => (None, None),
        "en-AU" => (Some("en"), Some("Australian spelling.")),
        "en-GB" => (Some("en"), Some("British spelling.")),
        "en-US" => (Some("en"), Some("American spelling.")),
        code => (Some(code), None),
    };

    let committed_streaming_text = Arc::new(Mutex::new(String::new()));
    let (partial_tx, streaming_task) = if streaming_enabled {
        crate::log_info!("⌨️ Streaming typewriter enabled for this recording");
        let (partial_tx, partial_rx) = mpsc::unbounded_channel();
        let task = spawn_streaming_typewriter(
            app_handle.clone(),
            config.clone(),
            partial_rx,
            committed_streaming_text.clone(),
            lang_code.map(ToString::to_string),
            prompt_hint.map(ToString::to_string),
        );
        (Some(partial_tx), Some(task))
    } else {
        if streaming_output_method != config::OutputMethod::Typewriter {
            crate::log_info!("⌨️ Streaming typewriter inactive: output method is not Typewriter");
        }
        (None, None)
    };

    let audio_data = match audio::record_audio_while_flag_with_partials(
        &is_recording,
        audio_engine,
        partial_tx,
    )
    .await
    {
        Ok(data) => data,
        Err(error) => {
            reset_status_on_exit().await;
            return Err(error);
        }
    };
    if let Some(task) = streaming_task {
        task.abort();
    }

    if audio_data.is_empty() {
        reset_status_on_exit().await;
        return Ok(());
    }
    if let Err(error) = validate_audio_duration(&audio_data) {
        crate::log_info!("⚠️ Audio validation failed: {}", error);
        reset_status_on_exit().await;
        return Ok(());
    }

    crate::app::status::emit_status_to_frontend("Transcribing").await;
    let (
        transcription_mode,
        api_key,
        api_url,
        api_model,
        debug_mode,
        enable_recording_logs,
    ) = {
        let config_guard = config.lock().unwrap();
        (
            config_guard.transcription_mode.clone(),
            config_guard.openai_api_key.clone(),
            config_guard.api_url.clone(),
            config_guard.api_model.clone(),
            config_guard.debug_mode,
            config_guard.enable_recording_logs,
        )
    };

    if debug_mode && enable_recording_logs {
        let debug_path = dirs::config_dir()
            .unwrap_or_default()
            .join("foss-voquill")
            .join("debug")
            .join(format!(
                "recording_{}.wav",
                ::chrono::Local::now().format("%Y%m%d_%H%M%S")
            ));

        if let Err(error) = std::fs::create_dir_all(debug_path.parent().unwrap()) {
            crate::log_info!("❌ Failed to create debug directory: {}", error);
        } else if let Err(error) = std::fs::write(&debug_path, &audio_data) {
            crate::log_info!("❌ Failed to save debug recording: {}", error);
        } else {
            crate::log_info!("🛡️ Debug recording saved to: {:?}", debug_path);
        }
    }

    crate::log_info!("📡 Transcription Mode: {:?}", transcription_mode);
    crate::log_info!("🌐 Language: {:?}, Hint: {:?}", lang_code, prompt_hint);

    let service: Box<dyn transcription::TranscriptionService + Send + Sync> =
        match transcription_mode {
            TranscriptionMode::API => Box::new(transcription::APITranscriptionService {
                api_key,
                api_url,
                api_model,
            })
                as Box<dyn transcription::TranscriptionService + Send + Sync>,
            TranscriptionMode::Local => {
                let (engine, model_size, accelerator, use_gpu) = {
                    let config_lock = config.lock().unwrap();
                    (
                        config_lock.local_engine.clone(),
                        config_lock.local_model_size.clone(),
                        config_lock.local_accelerator.clone(),
                        config_lock.enable_gpu,
                    )
                };

                match engine.as_str() {
                    "OpenVINO GenAI" => {
                        match openvino_whisper::OpenVinoWhisperService::new(
                            &model_size,
                            &accelerator,
                        ) {
                            Ok(service) => Box::new(service)
                                as Box<dyn transcription::TranscriptionService + Send + Sync>,
                            Err(error) => {
                                crate::log_info!(
                                    "❌ Failed to initialize OpenVINO Whisper: {}",
                                    error
                                );
                                reset_status_on_exit().await;
                                return Err(error.into());
                            }
                        }
                    }
                    _ => match local_whisper::LocalWhisperService::new(&model_size, use_gpu) {
                        Ok(service) => Box::new(service)
                            as Box<dyn transcription::TranscriptionService + Send + Sync>,
                        Err(error) => {
                            crate::log_info!("❌ Failed to initialize Local Whisper: {}", error);
                            reset_status_on_exit().await;
                            return Err(error.into());
                        }
                    },
                }
            }
        };

    let text = match service
        .transcribe(&audio_data, lang_code, prompt_hint)
        .await
    {
        Ok(text) => {
            crate::log_info!(
                "📝 Transcription received ({}): \"{}\"",
                service.service_name(),
                text
            );
            text
        }

        Err(error) => {
            crate::log_info!(
                "❌ Transcription failed ({}): {}",
                service.service_name(),
                error
            );
            reset_status_on_exit().await;
            return Err(error.into());
        }
    };

    if !text.trim().is_empty() {
        let _ = history::add_history_item(&text);
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }

        crate::app::status::emit_status_to_frontend("Typing").await;
        let (typing_speed, hold_duration, output_method, copy_on_typewriter) = {
            let config_guard = config.lock().unwrap();
            (
                config_guard.typing_speed_interval,
                config_guard.key_press_duration_ms,
                config_guard.output_method.clone(),
                config_guard.copy_on_typewriter,
            )
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        match output_method {
            config::OutputMethod::Typewriter => {
                if copy_on_typewriter {
                    if let Err(error) = typing::copy_to_clipboard(&text) {
                        crate::log_info!("❌ CLIPBOARD ERROR: {}", error);
                    }
                }
                let text_to_type = if streaming_enabled {
                    let committed = committed_streaming_text.lock().unwrap().clone();
                    match suffix_after_committed(&text, &committed) {
                        Some(suffix) => suffix.to_string(),
                        None => {
                            crate::log_info!(
                                "⚠️ Streaming typewriter final text was non-monotonic; skipping final type to avoid duplicate output. committed='{}', final='{}'",
                                committed,
                                text
                            );
                            String::new()
                        }
                    }
                } else {
                    text.clone()
                };

                if text_to_type.trim().is_empty() {
                    crate::log_info!("⌨️  Final typewriter output already satisfied by streaming partials");
                    reset_status_on_exit().await;
                    return Ok(());
                }

                crate::log_info!("⌨️  Forwarding text to hardware typing engine...");
                let state = app_handle.state::<crate::AppState>();
                if let Err(error) = state
                    .display_backend
                    .type_text_hardware(&app_handle, &text_to_type, typing_speed, hold_duration)
                    .await
                {
                    crate::log_info!("❌ TYPING ENGINE ERROR: {}", error);
                }
            }
            config::OutputMethod::Clipboard => {
                crate::log_info!("📋 Copying text to clipboard (Clipboard Mode)...");
                if let Err(error) = typing::copy_to_clipboard(&text) {
                    crate::log_info!("❌ CLIPBOARD ERROR: {}", error);
                }
            }
        }
    } else {
        crate::log_info!("ℹ️ Transcription was empty, skipping typing.");
    }

    reset_status_on_exit().await;
    Ok(())
}
