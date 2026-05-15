use crate::model_manager::ModelManager;
use crate::transcription::{TranscriptionError, TranscriptionService};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::Duration;

const WORKER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(240);

pub struct MlxWhisperService {
    model_path: PathBuf,
}

struct MlxWorker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    stderr_lines: Arc<Mutex<Vec<String>>>,
}

struct MlxWorkerPool;

#[derive(Debug, Serialize)]
struct WorkerRequest<'a> {
    audio_path: String,
    language: Option<&'a str>,
    prompt: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct WorkerResponse {
    ok: bool,
    text: Option<String>,
    error: Option<String>,
}

impl MlxWorkerPool {
    fn get() -> &'static Mutex<HashMap<String, MlxWorker>> {
        static WORKERS: OnceLock<Mutex<HashMap<String, MlxWorker>>> = OnceLock::new();
        WORKERS.get_or_init(|| Mutex::new(HashMap::new()))
    }
}

impl MlxWhisperService {
    pub fn new(model_size: &str) -> Result<Self, TranscriptionError> {
        if !cfg!(target_os = "macos") {
            return Err(TranscriptionError::ModelError(
                "MLX Whisper is only available on Apple Silicon macOS.".to_string(),
            ));
        }

        let model_manager = ModelManager::new().map_err(TranscriptionError::ModelError)?;
        let model_path = model_manager.get_model_path(model_size);

        if !model_manager.is_model_downloaded(model_size) {
            return Err(TranscriptionError::ModelError(format!(
                "MLX model {} not found. Please download it in settings.",
                model_size
            )));
        }

        Ok(Self { model_path })
    }
}

#[async_trait]
impl TranscriptionService for MlxWhisperService {
    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let audio_data = audio_data.to_vec();
        let model_path = self.model_path.clone();
        let language = language.map(ToString::to_string);
        let prompt = prompt.map(ToString::to_string);

        tokio::task::spawn_blocking(move || {
            transcribe_with_worker(
                &model_path,
                &audio_data,
                language.as_deref(),
                prompt.as_deref(),
            )
        })
        .await
        .map_err(|error| TranscriptionError::ModelError(error.to_string()))?
    }

    fn service_name(&self) -> &'static str {
        "MLX Whisper"
    }
}

fn transcribe_with_worker(
    model_path: &Path,
    audio_data: &[u8],
    language: Option<&str>,
    prompt: Option<&str>,
) -> Result<String, TranscriptionError> {
    let audio_path = write_temp_wav(audio_data)?;
    let output = send_worker_request(model_path, &audio_path, language, prompt);
    let _ = std::fs::remove_file(&audio_path);
    output
}

fn write_temp_wav(audio_data: &[u8]) -> Result<PathBuf, TranscriptionError> {
    let audio_path = std::env::temp_dir().join(format!(
        "voquill-mlx-{}.wav",
        chrono::Local::now()
            .timestamp_nanos_opt()
            .unwrap_or_default()
    ));
    std::fs::write(&audio_path, audio_data)
        .map_err(|error| TranscriptionError::AudioError(error.to_string()))?;
    Ok(audio_path)
}

fn send_worker_request(
    model_path: &Path,
    audio_path: &Path,
    language: Option<&str>,
    prompt: Option<&str>,
) -> Result<String, TranscriptionError> {
    let key = model_path.to_string_lossy().to_string();
    let mut workers = MlxWorkerPool::get()
        .lock()
        .map_err(|error| TranscriptionError::ModelError(error.to_string()))?;
    if !workers.contains_key(&key) {
        workers.insert(key.clone(), start_worker(model_path)?);
    }

    let worker = workers
        .get_mut(&key)
        .ok_or_else(|| TranscriptionError::ModelError("MLX worker was not created".to_string()))?;
    match worker.transcribe(audio_path, language, prompt) {
        Ok(text) => Ok(text),
        Err(error) => {
            let _ = worker.child.kill();
            workers.remove(&key);
            Err(error)
        }
    }
}

fn start_worker(model_path: &Path) -> Result<MlxWorker, TranscriptionError> {
    let mut child = Command::new("python3")
        .arg("-u")
        .arg("-c")
        .arg(PYTHON_WORKER)
        .arg(model_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            TranscriptionError::ModelError(format!(
                "Failed to start MLX Python worker. Install a native Apple Silicon Python 3 runtime with mlx-whisper: python3 -m pip install mlx-whisper. Error: {error}"
            ))
        })?;

    let stdin = child.stdin.take().ok_or_else(|| {
        TranscriptionError::ModelError("MLX worker stdin unavailable".to_string())
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        TranscriptionError::ModelError("MLX worker stdout unavailable".to_string())
    })?;
    let stderr_lines = Arc::new(Mutex::new(Vec::new()));
    if let Some(stderr) = child.stderr.take() {
        pipe_worker_stderr(stderr, stderr_lines.clone());
    }

    Ok(MlxWorker {
        child,
        stdin,
        stdout: BufReader::new(stdout),
        stderr_lines,
    })
}

fn pipe_worker_stderr(stderr: std::process::ChildStderr, stderr_lines: Arc<Mutex<Vec<String>>>) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            crate::log_info!("MLX worker: {}", line);
            if let Ok(mut lines) = stderr_lines.lock() {
                lines.push(line);
                if lines.len() > 40 {
                    lines.remove(0);
                }
            }
        }
    });
}

impl MlxWorker {
    fn transcribe(
        &mut self,
        audio_path: &Path,
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let request = WorkerRequest {
            audio_path: audio_path.to_string_lossy().to_string(),
            language,
            prompt,
        };
        let request_json = serde_json::to_string(&request)
            .map_err(|error| TranscriptionError::ModelError(error.to_string()))?;
        writeln!(self.stdin, "{}", request_json)
            .and_then(|_| self.stdin.flush())
            .map_err(|error| TranscriptionError::ModelError(error.to_string()))?;

        let mut response_json = String::new();
        let worker_process_id = self.child.id();
        let request_completed = Arc::new(AtomicBool::new(false));
        let watchdog_completed = Arc::clone(&request_completed);
        std::thread::spawn(move || {
            std::thread::sleep(WORKER_RESPONSE_TIMEOUT);
            if watchdog_completed.load(Ordering::SeqCst) {
                return;
            }

            crate::log_warn!(
                "MLX worker request exceeded {}s; terminating worker pid={}",
                WORKER_RESPONSE_TIMEOUT.as_secs(),
                worker_process_id
            );
            let _ = Command::new("kill")
                .args(["-TERM", &worker_process_id.to_string()])
                .status();
        });

        let read_result = self.stdout.read_line(&mut response_json);
        request_completed.store(true, Ordering::SeqCst);
        read_result.map_err(|error| TranscriptionError::ModelError(error.to_string()))?;

        if response_json.trim().is_empty() {
            let stderr = self.read_stderr();
            return Err(TranscriptionError::ModelError(
                if stderr.trim().is_empty() {
                    "MLX worker exited without a response".to_string()
                } else {
                    format!("MLX worker exited without a response: {}", stderr.trim())
                },
            ));
        }

        let response = serde_json::from_str::<WorkerResponse>(&response_json)
            .map_err(|error| TranscriptionError::ModelError(error.to_string()))?;
        if response.ok {
            return Ok(response.text.unwrap_or_default().trim().to_string());
        }

        Err(TranscriptionError::ModelError(
            response
                .error
                .unwrap_or_else(|| "MLX transcription failed".to_string()),
        ))
    }

    fn read_stderr(&self) -> String {
        self.stderr_lines
            .lock()
            .map(|lines| lines.join("\n"))
            .unwrap_or_default()
    }
}

const PYTHON_WORKER: &str = r#"
import inspect
import json
import sys
import traceback
from pathlib import Path

model_path = Path(sys.argv[1])

try:
    import mlx_whisper
except Exception as error:
    print(json.dumps({
        "ok": False,
        "error": "Failed to import mlx_whisper. Install it with: python3 -m pip install mlx-whisper. " + str(error),
    }), flush=True)
    sys.exit(1)

signature = inspect.signature(mlx_whisper.transcribe)
supported = signature.parameters

for line in sys.stdin:
    try:
        request = json.loads(line)
        kwargs = {"path_or_hf_repo": str(model_path)}
        if request.get("language") and "language" in supported:
            kwargs["language"] = request["language"]
        if request.get("prompt") and "initial_prompt" in supported:
            kwargs["initial_prompt"] = request["prompt"]

        result = mlx_whisper.transcribe(request["audio_path"], **kwargs)
        print(json.dumps({"ok": True, "text": result.get("text", "")}), flush=True)
    except Exception as error:
        print(json.dumps({
            "ok": False,
            "error": str(error) + "\n" + traceback.format_exc(),
        }), flush=True)
"#;
