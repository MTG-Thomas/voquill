use crate::transcription::TranscriptionService;
use crate::{history, model_manager, openvino_whisper, transcription};
use tauri::Emitter;

#[tauri::command]
pub async fn test_api_key(api_key: String, api_url: String) -> Result<bool, String> {
    transcription::test_api_key(&api_key, &api_url)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_available_engines() -> Result<Vec<String>, String> {
    Ok(model_manager::ModelManager::get_available_engines())
}

#[tauri::command]
pub async fn get_available_models() -> Result<Vec<model_manager::ModelInfo>, String> {
    Ok(model_manager::ModelManager::get_available_models())
}

#[tauri::command]
pub async fn check_model_status(
    model_size: String,
    _engine: Option<String>,
) -> Result<bool, String> {
    let manager = model_manager::ModelManager::new().map_err(|error| error.to_string())?;
    Ok(manager.is_model_downloaded(&model_size))
}

#[tauri::command]
pub async fn download_model(
    model_size: String,
    _engine: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let manager = model_manager::ModelManager::new().map_err(|error| error.to_string())?;

    manager
        .download_model(&model_size, move |progress| {
            let _ = app_handle.emit("model-download-progress", progress);
        })
        .await?;

    Ok(())
}

#[tauri::command]
pub async fn warm_up_model(
    model_size: String,
    engine: String,
    accelerator: Option<String>,
) -> Result<(), String> {
    if engine != "OpenVINO GenAI" {
        return Err(format!(
            "Warmup is only available for OpenVINO GenAI, not {}",
            engine
        ));
    }

    warm_up_openvino_model(&model_size, accelerator.as_deref()).await
}

pub(crate) async fn warm_up_openvino_model(
    model_size: &str,
    accelerator: Option<&str>,
) -> Result<(), String> {
    let accelerator = accelerator.unwrap_or("NPU");
    crate::log_info!(
        "🔥 OpenVINO warmup requested: model={}, accelerator={}",
        model_size,
        accelerator
    );

    let service = openvino_whisper::OpenVinoWhisperService::new(model_size, accelerator)
        .map_err(|error| error.to_string())?;

    service
        .transcribe(&create_silent_warmup_wav(), None, None)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())?;

    crate::log_info!(
        "✅ OpenVINO warmup complete: model={}, accelerator={}",
        model_size,
        accelerator
    );

    Ok(())
}

#[tauri::command]
pub fn get_current_status() -> String {
    crate::app::status::get_current_status()
}

#[tauri::command]
pub async fn get_history() -> Result<history::History, String> {
    history::load_history().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clear_history() -> Result<(), String> {
    history::clear_history().map_err(|error| error.to_string())
}

fn create_silent_warmup_wav() -> Vec<u8> {
    let sample_rate = 16_000u32;
    let channels = 1u16;
    let bits_per_sample = 16u16;
    let duration_samples = sample_rate / 2;

    let mut wav_data = Vec::new();
    wav_data.extend_from_slice(b"RIFF");
    let file_size = 36 + duration_samples * channels as u32 * bits_per_sample as u32 / 8;
    wav_data.extend_from_slice(&(file_size - 8).to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes());
    wav_data.extend_from_slice(&1u16.to_le_bytes());
    wav_data.extend_from_slice(&channels.to_le_bytes());
    wav_data.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    wav_data.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * bits_per_sample / 8;
    wav_data.extend_from_slice(&block_align.to_le_bytes());
    wav_data.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav_data.extend_from_slice(b"data");
    let data_size = duration_samples * channels as u32 * bits_per_sample as u32 / 8;
    wav_data.extend_from_slice(&data_size.to_le_bytes());

    for _ in 0..duration_samples {
        wav_data.extend_from_slice(&0i16.to_le_bytes());
    }

    wav_data
}
