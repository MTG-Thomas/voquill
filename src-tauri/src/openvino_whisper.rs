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

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const WORKER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(240);

pub struct OpenVinoWhisperService {
    model_path: PathBuf,
    device: String,
}

struct OpenVinoWorker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    stderr_lines: Arc<Mutex<Vec<String>>>,
}

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

impl LocalWorkerPool {
    fn get() -> &'static Mutex<HashMap<String, OpenVinoWorker>> {
        static WORKERS: OnceLock<Mutex<HashMap<String, OpenVinoWorker>>> = OnceLock::new();
        WORKERS.get_or_init(|| Mutex::new(HashMap::new()))
    }
}

struct LocalWorkerPool;

impl OpenVinoWhisperService {
    pub fn new(model_size: &str, device: &str) -> Result<Self, TranscriptionError> {
        let model_manager = ModelManager::new().map_err(TranscriptionError::ModelError)?;
        let model_path = model_manager.get_model_path(model_size);

        if !model_manager.is_model_downloaded(model_size) {
            return Err(TranscriptionError::ModelError(format!(
                "OpenVINO model {} not found. Please download it in settings.",
                model_size
            )));
        }

        Ok(Self {
            model_path,
            device: normalize_device(device),
        })
    }
}

#[async_trait]
impl TranscriptionService for OpenVinoWhisperService {
    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let model_path = self.model_path.clone();
        let device = self.device.clone();
        let audio_data = audio_data.to_vec();
        let language = language.map(ToString::to_string);
        let prompt = prompt.map(ToString::to_string);

        tokio::task::spawn_blocking(move || {
            transcribe_with_worker(
                &model_path,
                &device,
                &audio_data,
                language.as_deref(),
                prompt.as_deref(),
            )
        })
        .await
        .map_err(|error| TranscriptionError::ModelError(error.to_string()))?
    }

    fn service_name(&self) -> &'static str {
        "OpenVINO GenAI"
    }
}

fn normalize_device(device: &str) -> String {
    match device.trim().to_uppercase().as_str() {
        "CPU" | "GPU" | "AUTO" => device.trim().to_uppercase(),
        _ => "NPU".to_string(),
    }
}

fn transcribe_with_worker(
    model_path: &Path,
    device: &str,
    audio_data: &[u8],
    language: Option<&str>,
    prompt: Option<&str>,
) -> Result<String, TranscriptionError> {
    let audio_path = write_temp_wav(audio_data)?;
    let output = send_worker_request(model_path, device, &audio_path, language, prompt);
    let _ = std::fs::remove_file(&audio_path);
    output
}

fn write_temp_wav(audio_data: &[u8]) -> Result<PathBuf, TranscriptionError> {
    let audio_path = std::env::temp_dir().join(format!(
        "voquill-openvino-{}.wav",
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
    device: &str,
    audio_path: &Path,
    language: Option<&str>,
    prompt: Option<&str>,
) -> Result<String, TranscriptionError> {
    let key = format!("{}|{}", model_path.display(), device);
    let mut workers = LocalWorkerPool::get()
        .lock()
        .map_err(|error| TranscriptionError::ModelError(error.to_string()))?;
    if !workers.contains_key(&key) {
        workers.insert(key.clone(), start_worker(model_path, device)?);
    }

    let worker = workers.get_mut(&key).ok_or_else(|| {
        TranscriptionError::ModelError("OpenVINO worker was not created".to_string())
    })?;
    match worker.transcribe(audio_path, language, prompt) {
        Ok(text) => Ok(text),
        Err(error) => {
            let _ = worker.child.kill();
            workers.remove(&key);
            Err(error)
        }
    }
}

fn start_worker(model_path: &Path, device: &str) -> Result<OpenVinoWorker, TranscriptionError> {
    let python = resolve_python_runtime();
    let mut command = Command::new(&python.executable);
    hide_console_window(&mut command);
    for argument in &python.arguments {
        command.arg(argument);
    }
    if python
        .executable
        .to_string_lossy()
        .eq_ignore_ascii_case("py")
        && python.arguments.is_empty()
    {
        command.arg("-3");
    }

    let mut child = command
        .arg("-u")
        .arg("-c")
        .arg(PYTHON_WORKER)
        .arg(model_path)
        .arg(device)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            TranscriptionError::ModelError(format!(
                "Failed to start OpenVINO Python runtime '{}': {}",
                python.executable.display(),
                error
            ))
        })?;

    let stdin = child.stdin.take().ok_or_else(|| {
        TranscriptionError::ModelError("OpenVINO worker stdin unavailable".to_string())
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        TranscriptionError::ModelError("OpenVINO worker stdout unavailable".to_string())
    })?;
    let stderr_lines = Arc::new(Mutex::new(Vec::new()));
    if let Some(stderr) = child.stderr.take() {
        pipe_worker_stderr(stderr, device.to_string(), stderr_lines.clone());
    }

    Ok(OpenVinoWorker {
        child,
        stdin,
        stdout: BufReader::new(stdout),
        stderr_lines,
    })
}

fn pipe_worker_stderr(
    stderr: std::process::ChildStderr,
    device: String,
    stderr_lines: Arc<Mutex<Vec<String>>>,
) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            crate::log_info!("OpenVINO worker [{}]: {}", device, line);
            if let Ok(mut lines) = stderr_lines.lock() {
                lines.push(line);
                if lines.len() > 40 {
                    lines.remove(0);
                }
            }
        }
    });
}

fn terminate_process_tree(process_id: u32) {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("taskkill");
        hide_console_window(&mut command);
        let _ = command
            .args(["/PID", &process_id.to_string(), "/T", "/F"])
            .status();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &process_id.to_string()])
            .status();
    }
}

#[cfg(target_os = "windows")]
fn hide_console_window(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_console_window(_command: &mut Command) {}

impl OpenVinoWorker {
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
                "OpenVINO worker request exceeded {}s; terminating worker process tree pid={}",
                WORKER_RESPONSE_TIMEOUT.as_secs(),
                worker_process_id
            );
            terminate_process_tree(worker_process_id);
        });

        let read_result = self.stdout.read_line(&mut response_json);
        request_completed.store(true, Ordering::SeqCst);
        read_result.map_err(|error| TranscriptionError::ModelError(error.to_string()))?;

        if response_json.trim().is_empty() {
            let stderr = self.read_stderr();
            return Err(TranscriptionError::ModelError(
                if stderr.trim().is_empty() {
                    "OpenVINO worker exited without a response".to_string()
                } else {
                    format!(
                        "OpenVINO worker exited without a response: {}",
                        stderr.trim()
                    )
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
                .unwrap_or_else(|| "OpenVINO transcription failed".to_string()),
        ))
    }

    fn read_stderr(&mut self) -> String {
        self.stderr_lines
            .lock()
            .map(|lines| lines.join("\n"))
            .unwrap_or_default()
    }
}

struct PythonRuntime {
    executable: PathBuf,
    arguments: Vec<String>,
}

fn resolve_python_runtime() -> PythonRuntime {
    if let Ok(configured_python) = std::env::var("VOQUILL_OPENVINO_PYTHON") {
        return PythonRuntime {
            executable: PathBuf::from(configured_python),
            arguments: Vec::new(),
        };
    }

    if let Some(local_data_dir) = dirs::data_local_dir() {
        let runtime_python = local_data_dir
            .join("Voquill")
            .join("openvino-runtime")
            .join(".venv")
            .join("Scripts")
            .join("python.exe");
        if runtime_python.exists() {
            return PythonRuntime {
                executable: runtime_python,
                arguments: Vec::new(),
            };
        }
    }

    PythonRuntime {
        executable: PathBuf::from("py"),
        arguments: vec!["-3".to_string()],
    }
}

const PYTHON_WORKER: &str = r#"
import json
import sys
import time
import traceback
import wave
from pathlib import Path

import numpy as np

model_path = Path(sys.argv[1])
device = sys.argv[2]
worker_started_at = time.perf_counter()

def log_phase(message):
    elapsed = time.perf_counter() - worker_started_at
    print(f"{elapsed:.3f}s {message}", file=sys.stderr, flush=True)

log_phase(f"worker started model={model_path.name} device={device}")

phase_started_at = time.perf_counter()
import openvino_genai as ov_genai
log_phase(f"imported openvino_genai in {time.perf_counter() - phase_started_at:.3f}s")

def read_wav_mono_16k(audio_path):
    with wave.open(str(audio_path), "rb") as wav:
        channels = wav.getnchannels()
        rate = wav.getframerate()
        width = wav.getsampwidth()
        frames = wav.readframes(wav.getnframes())

    if width != 2:
        raise RuntimeError(f"Expected 16-bit PCM WAV, got sample width {width}")

    samples = np.frombuffer(frames, dtype=np.int16).astype(np.float32) / 32768.0
    if channels > 1:
        samples = samples.reshape(-1, channels).mean(axis=1)
    if rate != 16000:
        old_x = np.arange(samples.shape[0], dtype=np.float64) / rate
        new_len = int(round(samples.shape[0] * 16000 / rate))
        new_x = np.arange(new_len, dtype=np.float64) / 16000
        samples = np.interp(new_x, old_x, samples).astype(np.float32)
    return samples

kwargs = {"STATIC_PIPELINE": True} if device == "NPU" else {}
log_phase(f"constructing WhisperPipeline kwargs={kwargs}")
phase_started_at = time.perf_counter()
pipeline = ov_genai.WhisperPipeline(str(model_path), device, **kwargs)
log_phase(f"constructed WhisperPipeline in {time.perf_counter() - phase_started_at:.3f}s")

for line in sys.stdin:
    try:
        request_started_at = time.perf_counter()
        request = json.loads(line)
        audio_path = Path(request["audio_path"])
        language = request.get("language")
        phase_started_at = time.perf_counter()
        samples = read_wav_mono_16k(audio_path)
        log_phase(f"decoded audio in {time.perf_counter() - phase_started_at:.3f}s")

        generation_kwargs = {}
        if language and not model_path.name.endswith(".en-int8-ov"):
            generation_kwargs["language"] = language

        phase_started_at = time.perf_counter()
        result = pipeline.generate(samples, **generation_kwargs)
        log_phase(
            f"generated transcription in {time.perf_counter() - phase_started_at:.3f}s "
            f"(request total {time.perf_counter() - request_started_at:.3f}s)"
        )
        print(json.dumps({"ok": True, "text": getattr(result, "text", str(result)).strip()}), flush=True)
    except Exception as error:
        log_phase(f"request failed: {error}")
        print(json.dumps({"ok": False, "error": f"{error}\n{traceback.format_exc()}"}), flush=True)
"#;

#[cfg(test)]
mod tests {
    use super::OpenVinoWhisperService;
    use crate::transcription::TranscriptionService;

    #[tokio::test]
    #[ignore = "requires the app-local OpenVINO runtime and downloaded OpenVINO base model"]
    async fn openvino_worker_transcribes_silence_when_runtime_is_available() {
        let service = OpenVinoWhisperService::new("openvino-whisper-base.en-int8", "NPU")
            .expect("OpenVINO base model should be downloaded");

        service
            .transcribe(&create_silent_wav(), Some("en"), None)
            .await
            .expect("OpenVINO worker should transcribe without failing");
    }

    fn create_silent_wav() -> Vec<u8> {
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
}
