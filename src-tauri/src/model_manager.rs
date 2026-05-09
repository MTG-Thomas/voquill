use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub engine: String,
    pub size: String,
    pub file_size: u64,
    pub download_url: String,
    pub sha256: String,
    pub label: String,
    pub description: String,
    pub recommended: bool,
    pub artifact: ModelArtifact,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelArtifact {
    GgmlFile,
    OpenVinoSnapshot,
}

pub struct ModelManager {
    pub models_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceModelResponse {
    siblings: Vec<HuggingFaceSibling>,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceSibling {
    rfilename: String,
    size: Option<u64>,
}

impl ModelManager {
    pub fn new() -> Result<Self, String> {
        let models_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("foss-voquill")
            .join("models");

        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
        }

        Ok(Self { models_dir })
    }

    pub fn get_available_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                engine: "Whisper.cpp".to_string(),
                size: "tiny.en".to_string(),
                label: "Tiny (English)".to_string(),
                file_size: 77_600_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin".to_string(),
                sha256: "be07098a4cc50130a511ca096303ad371c513297a7d4a093047d9ca4378f8776".to_string(),
                description: "Lightning fast, best for simple commands.".to_string(),
                recommended: false,
                artifact: ModelArtifact::GgmlFile,
            },
            ModelInfo {
                engine: "Whisper.cpp".to_string(),
                size: "distil-small.en".to_string(),
                label: "Distil-Small (English)".to_string(),
                file_size: 175_000_000,
                download_url: "https://huggingface.co/distil-whisper/distil-small.en/resolve/main/ggml-distil-small.en.bin".to_string(),
                sha256: "e8a676964fd3f78b021a385f078a18863712ca10fdc907a685eee9c0e71d7a62".to_string(),
                description: "Perfect balance of speed and high accuracy.".to_string(),
                recommended: true,
                artifact: ModelArtifact::GgmlFile,
            },
            ModelInfo {
                engine: "Whisper.cpp".to_string(),
                size: "base.en".to_string(),
                label: "Base (English)".to_string(),
                file_size: 147_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin".to_string(),
                sha256: "60ed30914c83ad34005b63359d992f802773d57864f7df26e95261895697d74d".to_string(),
                description: "Standard choice for general dictation.".to_string(),
                recommended: false,
                artifact: ModelArtifact::GgmlFile,
            },
            ModelInfo {
                engine: "Whisper.cpp".to_string(),
                size: "small.en".to_string(),
                label: "Small (English)".to_string(),
                file_size: 483_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin".to_string(),
                sha256: "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1".to_string(),
                description: "Great accuracy for complex vocabulary.".to_string(),
                recommended: false,
                artifact: ModelArtifact::GgmlFile,
            },
            ModelInfo {
                engine: "Whisper.cpp".to_string(),
                size: "medium.en".to_string(),
                label: "Medium (English)".to_string(),
                file_size: 1_500_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin".to_string(),
                sha256: "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1".to_string(),
                description: "Highest accuracy. Needs a powerful computer or GPU.".to_string(),
                recommended: false,
                artifact: ModelArtifact::GgmlFile,
            },
            ModelInfo {
                engine: "OpenVINO GenAI".to_string(),
                size: "openvino-whisper-tiny.en-int8".to_string(),
                label: "Tiny English INT8".to_string(),
                file_size: 46_400_000,
                download_url: "OpenVINO/whisper-tiny.en-int8-ov".to_string(),
                sha256: "".to_string(),
                description: "Fast Intel CPU/GPU/NPU model for short English dictation.".to_string(),
                recommended: false,
                artifact: ModelArtifact::OpenVinoSnapshot,
            },
            ModelInfo {
                engine: "OpenVINO GenAI".to_string(),
                size: "openvino-whisper-base.en-int8".to_string(),
                label: "Base English INT8".to_string(),
                file_size: 80_700_000,
                download_url: "OpenVINO/whisper-base.en-int8-ov".to_string(),
                sha256: "".to_string(),
                description: "Balanced Intel CPU/GPU/NPU model for English dictation.".to_string(),
                recommended: true,
                artifact: ModelArtifact::OpenVinoSnapshot,
            },
            ModelInfo {
                engine: "OpenVINO GenAI".to_string(),
                size: "openvino-whisper-small.en-int8".to_string(),
                label: "Small English INT8".to_string(),
                file_size: 244_000_000,
                download_url: "OpenVINO/whisper-small.en-int8-ov".to_string(),
                sha256: "".to_string(),
                description: "More accurate Intel CPU/GPU/NPU model for English dictation.".to_string(),
                recommended: false,
                artifact: ModelArtifact::OpenVinoSnapshot,
            },
            ModelInfo {
                engine: "OpenVINO GenAI".to_string(),
                size: "openvino-whisper-large-v3-turbo-int8".to_string(),
                label: "Large v3 Turbo INT8 (Experimental)".to_string(),
                file_size: 820_000_000,
                download_url: "FluidInference/whisper-large-v3-turbo-int8-ov-npu".to_string(),
                sha256: "".to_string(),
                description: "Experimental high-accuracy multilingual NPU model. Expect a slower cold load.".to_string(),
                recommended: false,
                artifact: ModelArtifact::OpenVinoSnapshot,
            },
        ]
    }

    pub fn get_available_engines() -> Vec<String> {
        let mut engines: Vec<String> = Self::get_available_models()
            .iter()
            .map(|m| m.engine.clone())
            .collect();
        engines.sort();
        engines.dedup();
        engines
    }

    pub fn get_model_path(&self, model_size: &str) -> PathBuf {
        if let Some(snapshot_name) = model_size.strip_prefix("openvino-whisper-") {
            return self
                .models_dir
                .join("openvino")
                .join(format!("whisper-{}-ov", snapshot_name));
        }

        self.models_dir.join(format!("ggml-{}.bin", model_size))
    }

    pub fn is_model_downloaded(&self, model_size: &str) -> bool {
        let model_path = self.get_model_path(model_size);
        if model_size.starts_with("openvino-whisper-") {
            return model_path.join("openvino_encoder_model.xml").exists()
                && model_path.join("openvino_decoder_model.xml").exists();
        }

        model_path.exists()
    }

    pub async fn download_model<F>(
        &self,
        model_size: &str,
        progress_callback: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(f64) + Send + 'static,
    {
        let models = Self::get_available_models();
        let model_info = models
            .iter()
            .find(|m| m.size == model_size)
            .ok_or_else(|| format!("Model size {} not found", model_size))?;

        let path = self.get_model_path(model_size);

        if model_info.artifact == ModelArtifact::OpenVinoSnapshot {
            return self
                .download_openvino_snapshot(model_info, path, progress_callback)
                .await;
        }

        let client = reqwest::Client::new();
        let mut response = client
            .get(&model_info.download_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let total_size = response.content_length().unwrap_or(model_info.file_size);
        let mut downloaded: u64 = 0;
        let mut last_reported_progress: f64 = -1.0;

        let mut file = tokio::fs::File::create(&path)
            .await
            .map_err(|e| e.to_string())?;

        use tokio::io::AsyncWriteExt;
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;

            let progress = (downloaded as f64 / total_size as f64) * 100.0;

            // Only report progress if it has increased by at least 0.5%
            // to prevent saturating the Tauri IPC bridge and freezing the UI
            if progress - last_reported_progress >= 0.5 || progress >= 100.0 {
                progress_callback(progress);
                last_reported_progress = progress;
            }
        }

        file.flush().await.map_err(|e| e.to_string())?;
        Ok(path)
    }

    async fn download_openvino_snapshot<F>(
        &self,
        model_info: &ModelInfo,
        path: PathBuf,
        progress_callback: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(f64) + Send + 'static,
    {
        let client = reqwest::Client::new();
        let repo_id = &model_info.download_url;
        let api_url = format!("https://huggingface.co/api/models/{}", repo_id);
        let response = client
            .get(api_url)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<HuggingFaceModelResponse>()
            .await
            .map_err(|error| error.to_string())?;

        tokio::fs::create_dir_all(&path)
            .await
            .map_err(|error| error.to_string())?;

        let files: Vec<HuggingFaceSibling> = response
            .siblings
            .into_iter()
            .filter(|sibling| !sibling.rfilename.starts_with("."))
            .collect();
        let total_size: u64 = files
            .iter()
            .filter_map(|sibling| sibling.size)
            .sum::<u64>()
            .max(model_info.file_size);
        let mut downloaded = 0u64;
        let mut last_reported_progress = -1.0f64;

        for sibling in files {
            if sibling.rfilename.contains("..")
                || sibling.rfilename.starts_with('/')
                || sibling.rfilename.starts_with('\\')
            {
                return Err(format!(
                    "Unsafe model filename returned by Hugging Face: {}",
                    sibling.rfilename
                ));
            }

            let destination = path.join(&sibling.rfilename);
            if let Some(parent) = destination.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|error| error.to_string())?;
            }

            let file_url = format!(
                "https://huggingface.co/{}/resolve/main/{}",
                repo_id, sibling.rfilename
            );
            let mut response = client
                .get(file_url)
                .send()
                .await
                .map_err(|error| error.to_string())?
                .error_for_status()
                .map_err(|error| error.to_string())?;

            let mut file = tokio::fs::File::create(&destination)
                .await
                .map_err(|error| error.to_string())?;
            use tokio::io::AsyncWriteExt;
            while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
                file.write_all(&chunk)
                    .await
                    .map_err(|error| error.to_string())?;
                downloaded += chunk.len() as u64;

                let progress = (downloaded as f64 / total_size as f64) * 100.0;
                if progress - last_reported_progress >= 0.5 || progress >= 100.0 {
                    progress_callback(progress.min(100.0));
                    last_reported_progress = progress;
                }
            }

            file.flush().await.map_err(|error| error.to_string())?;
        }

        progress_callback(100.0);
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::ModelManager;

    #[test]
    fn openvino_models_are_exposed_as_a_separate_engine() {
        let models = ModelManager::get_available_models();

        assert!(models.iter().any(|model| model.engine == "OpenVINO GenAI"));
        assert!(ModelManager::get_available_engines()
            .iter()
            .any(|engine| engine == "OpenVINO GenAI"));
    }

    #[test]
    fn openvino_models_use_directory_paths_instead_of_ggml_files() {
        let manager = ModelManager {
            models_dir: "models".into(),
        };

        assert_eq!(
            manager
                .get_model_path("openvino-whisper-base.en-int8")
                .to_string_lossy(),
            "models\\openvino\\whisper-base.en-int8-ov"
        );
    }

    #[test]
    fn openvino_candidate_models_include_small_and_turbo() {
        let models = ModelManager::get_available_models();

        assert!(models
            .iter()
            .any(|model| model.size == "openvino-whisper-small.en-int8"));
        assert!(models
            .iter()
            .any(|model| model.size == "openvino-whisper-large-v3-turbo-int8"));
    }
}
