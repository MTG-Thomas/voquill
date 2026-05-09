# Windows ML Model Catalog Evaluation

## Decision

Voquill should not move the current local model lifecycle wholesale to Windows ML Model Catalog yet.

The current Intel NPU path is OpenVINO GenAI over OpenVINO snapshot directories, launched through an app-local Python worker. Windows ML Model Catalog can download model files to a shared per-user location and return local paths, but the Windows ML ecosystem is centered on ONNX Runtime execution providers. Using the catalog for the current OpenVINO GenAI models would give us a second model manifest and hosting surface without proving that OpenVINO GenAI can consume catalog-managed paths with the same performance, cache behavior, and update guarantees.

Recommended path: defer adoption for the active OpenVINO GenAI path. Run a separate Windows ML/ONNX spike only if we choose to add ONNX variants. Keep GGML and OpenVINO-native models in the current runtime-managed cache until a Windows ML-compatible model path proves equal or better on the Intel NPU.

## Current State

### Model Inventory

The model registry lives in [src-tauri/src/model_manager.rs](../src-tauri/src/model_manager.rs). `ModelInfo` defines engine, logical size, expected size, download URL, hash, UI label, recommendation state, and artifact type. `ModelArtifact` currently has two variants: `GgmlFile` and `OpenVinoSnapshot`.

Current model families:

| Engine | Model id | Source | Stored as | Expected size |
| --- | --- | --- | --- | ---: |
| Whisper.cpp | `tiny.en` | Hugging Face `ggerganov/whisper.cpp` | `ggml-tiny.en.bin` | 77.6 MB |
| Whisper.cpp | `distil-small.en` | Hugging Face `distil-whisper/distil-small.en` | `ggml-distil-small.en.bin` | 175 MB |
| Whisper.cpp | `base.en` | Hugging Face `ggerganov/whisper.cpp` | `ggml-base.en.bin` | 147 MB |
| Whisper.cpp | `small.en` | Hugging Face `ggerganov/whisper.cpp` | `ggml-small.en.bin` | 483 MB |
| Whisper.cpp | `medium.en` | Hugging Face `ggerganov/whisper.cpp` | `ggml-medium.en.bin` | 1.5 GB |
| OpenVINO GenAI | `openvino-whisper-tiny.en-int8` | Hugging Face `OpenVINO/whisper-tiny.en-int8-ov` | snapshot directory | 46.4 MB |
| OpenVINO GenAI | `openvino-whisper-base.en-int8` | Hugging Face `OpenVINO/whisper-base.en-int8-ov` | snapshot directory | 80.7 MB |
| OpenVINO GenAI | `openvino-whisper-small.en-int8` | Hugging Face `OpenVINO/whisper-small.en-int8-ov` | snapshot directory | 244 MB |
| OpenVINO GenAI | `openvino-whisper-large-v3-turbo-int8` | Hugging Face `FluidInference/whisper-large-v3-turbo-int8-ov-npu` | snapshot directory | 820 MB estimate |

Live disk usage on this laptop:

| Path | Current size |
| --- | ---: |
| `%APPDATA%\foss-voquill\models` | 3050.9 MB |
| `%APPDATA%\foss-voquill\models\openvino` | 1267.6 MB |
| `%APPDATA%\foss-voquill\models\openvino\whisper-base.en-int8-ov` | 80.7 MB |
| `%APPDATA%\foss-voquill\models\openvino\whisper-small.en-int8-ov` | 244.8 MB |
| `%APPDATA%\foss-voquill\models\openvino\whisper-large-v3-turbo-int8-ov` | 942.1 MB |
| `%APPDATA%\foss-voquill\models\ggml-distil-small.en.bin` | 320.6 MB |
| `%APPDATA%\foss-voquill\models\ggml-medium.en.bin` | 1462.7 MB |
| `%LOCALAPPDATA%\Voquill\openvino-runtime` | 308.5 MB |

The build cache at `C:\t\v` is separate from model lifecycle and was 6933.7 MB during this evaluation.

### Storage and Lifecycle

`ModelManager::new()` creates the model root under `dirs::config_dir()\foss-voquill\models`. On Windows that resolves to `%APPDATA%\foss-voquill\models`. This is user-profile-local and app-private by convention, not machine-wide and not shared with other apps.

`get_model_path()` maps Whisper.cpp models to single `ggml-<size>.bin` files and OpenVINO models to `%APPDATA%\foss-voquill\models\openvino\whisper-<variant>-ov`.

`is_model_downloaded()` checks file existence for GGML models. For OpenVINO snapshots it checks for `openvino_encoder_model.xml` and `openvino_decoder_model.xml`.

`download_model()` downloads GGML models as a single file from Hugging Face and emits Tauri `model-download-progress`. For OpenVINO snapshots, `download_openvino_snapshot()` calls the Hugging Face model API, enumerates siblings, filters dotfiles, rejects unsafe filenames, recreates subdirectories, and downloads every sibling file into the model directory.

`reset_application_to_defaults()` deletes `%APPDATA%\foss-voquill\models` with `remove_dir_all`, recreates it, then clears debug logs/history/config. It does not remove `%LOCALAPPDATA%\Voquill\openvino-runtime`.

There is no installer-bundled model in the current code. Models are downloaded on user action through the Settings/onboarding UI. Upgrades should reuse the same app-private model directory as long as the app identifier and config path remain stable. If model ids or path mapping change, old files can become orphaned because there is no model manifest reconciliation or garbage collector.

## Runtime Path

### Whisper.cpp

`LocalWhisperService` in [src-tauri/src/local_whisper.rs](../src-tauri/src/local_whisper.rs) uses `whisper-rs` with the `vulkan` feature enabled in [src-tauri/Cargo.toml](../src-tauri/Cargo.toml). It passes the local GGML path into `WhisperContext::new_with_params()`, creates a state, and runs `FullParams` over decoded WAV samples.

Consumed format: single GGML `.bin` files. These are not ONNX, ORT format, Windows ML package assets, or OpenVINO IR.

### OpenVINO GenAI / Intel NPU

`OpenVinoWhisperService` in [src-tauri/src/openvino_whisper.rs](../src-tauri/src/openvino_whisper.rs) resolves a model directory through `ModelManager`, verifies it is downloaded, and uses a persistent worker keyed by `model_path|device`.

The worker is a Python subprocess. Runtime discovery prefers:

1. `VOQUILL_OPENVINO_PYTHON`
2. `%LOCALAPPDATA%\Voquill\openvino-runtime\.venv\Scripts\python.exe`
3. `py -3`

The Python code imports `openvino_genai`, constructs `ov_genai.WhisperPipeline(str(model_path), device, **kwargs)`, and passes `STATIC_PIPELINE=True` when the selected device is `NPU`.

Consumed format: OpenVINO GenAI snapshot directory. The app explicitly checks for OpenVINO IR XML files. Actual snapshots also include OpenVINO BIN files and tokenizer/config sidecars.

Runtime installed by [scripts/setup-openvino-runtime.ps1](../scripts/setup-openvino-runtime.ps1):

- `openvino`
- `openvino-genai`
- `openvino-tokenizers`
- `numpy`

The current package script runs this with `-UseNightly`.

The NPU path may generate driver/runtime-specific optimized artifacts internally. This evaluation did not find a repo-managed compiled artifact directory. We should treat OpenVINO/NPU compilation caches as runtime-private and potentially tied to OpenVINO, driver, and NPU firmware versions unless OpenVINO documentation proves otherwise.

## Windows ML Model Catalog Fit

Microsoft describes Model Catalog as a way for an app or library to dynamically download large AI model files to a shared on-device location from model catalog sources, with compatibility filtering and sharing when multiple apps request the same SHA256-identical model. The catalog source is JSON and can be hosted over HTTPS or referenced as a local file.

Important doc facts verified during this evaluation:

- Model Catalog supports file-based models and package-based models.
- File entries include `name`, optional `uri`, and required `sha256`.
- A model definition requires `id`, `name`, `version`, `publisher`, `executionProviders`, `license`, `licenseUri`, and `uri`.
- `CatalogModelInstance.GetInstanceAsync()` returns `ModelPaths`, so apps can pass paths into their own inference code.
- Catalog storage is user-specific, and sharing depends on identical SHA256 hashes.
- Windows ML execution providers include CPU and DirectML, and dynamically downloadable EPs on Windows 11 24H2+ include OpenVINO for Intel hardware.
- Windows ML / ONNX Runtime migration guidance assumes ONNX Runtime APIs and ONNX-compatible models.

References:

- [Windows ML Model Catalog overview](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/model-catalog/overview)
- [Get started with Windows ML Model Catalog APIs](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/model-catalog/get-started)
- [Model Catalog Source schema reference](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/model-catalog/model-catalog-source)
- [Windows ML execution providers](https://learn.microsoft.com/nb-no/windows/ai/new-windows-ml/supported-execution-providers)
- [Install Windows ML execution providers](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/initialize-execution-providers)
- [Migrate from standalone ONNX Runtime to Windows ML's ONNX Runtime](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/migrate-to-windows-ml)

### Compatibility by Current Model Type

| Model type | Catalog can describe/download files? | Current runtime can use catalog path? | Recommended? |
| --- | --- | --- | --- |
| GGML `.bin` for `whisper-rs` | Likely yes as arbitrary file entries, if hosted with SHA256 | Likely yes if `ModelPaths[0]` is stable for the file | No near-term gain; not Windows ML compatible |
| OpenVINO GenAI snapshot | Possibly yes as many file entries with checksums | Unknown; likely possible if the returned paths reconstruct the snapshot directory layout | Defer until a PoC confirms directory layout and GenAI compatibility |
| ONNX Whisper model | Yes | Not with current runtime; requires ONNX Runtime or Windows ML runtime code | Good candidate for a separate spike |
| Windows ML EP package | Yes, but that is EP lifecycle, not model lifecycle | Not relevant until ONNX Runtime is used | Separate from model cache decision |

The catalog is not a universal Hugging Face snapshot cache. It can describe multiple files, but Voquill would need to host and pin every required file with SHA256 values, preserve sidecar layout, and maintain catalog ids/versions. The current downloader can pull a whole Hugging Face snapshot from the model API without precomputing every file hash.

### Compatibility Matrix Expressiveness

The catalog can express execution provider compatibility at a high level, for example `OpenVINOExecutionProvider`, `DmlExecutionProvider`, or `CPUExecutionProvider`. That is not enough by itself for Voquill's current OpenVINO GenAI path because our real compatibility matrix includes:

- OpenVINO GenAI availability in the Python environment.
- OpenVINO device names: `NPU`, `GPU`, `CPU`, `AUTO`.
- `STATIC_PIPELINE=True` for NPU.
- Intel NPU driver and firmware behavior.
- Model family-specific snapshot layout.
- Quantization and precision (`INT8`).
- Cold compile/load behavior and runtime-private caches.

Windows ML EP compatibility is useful if Voquill migrates to ONNX Runtime with the Windows ML OpenVINO EP. It does not prove that the existing OpenVINO GenAI snapshot path is equivalent.

## Proof-of-Concept Feasibility

A repo-integrated PoC is not safe to add yet.

Reasons:

- The app is Rust/Tauri. The Model Catalog examples use `Microsoft.Windows.AI.MachineLearning` through Windows App SDK projections, primarily C#, C++/WinRT, C/C++, and Python examples. This repo currently uses the Rust `windows` crate for Win32 APIs, not Windows App SDK WinRT package integration.
- Adding Windows App SDK/WinRT bindings would be more than a storage experiment. It would introduce packaging and deployment questions unrelated to the current OpenVINO GenAI worker.
- A useful PoC must verify returned `ModelPaths`, directory shape, offline behavior, corruption behavior, and OpenVINO GenAI loading. A JSON-only stub cannot prove the question that matters.

The smallest credible PoC should be a separate spike, gated by `VOQUILL_USE_WINDOWS_ML_CATALOG=1`, and should use a tiny ONNX model or a very small OpenVINO snapshot whose files are hosted with SHA256 values.

Illustrative local catalog source shape:

```json
{
  "models": [
    {
      "id": "voquill-whisper-base-openvino-int8-npu",
      "name": "voquill-whisper-base",
      "version": "2026.05.09",
      "publisher": "Voquill",
      "executionProviders": [
        { "name": "OpenVINOExecutionProvider" },
        { "name": "CPUExecutionProvider" }
      ],
      "modelSizeBytes": 80700000,
      "license": "Apache-2.0",
      "licenseUri": "https://huggingface.co/OpenVINO/whisper-base.en-int8-ov",
      "uri": "https://example.invalid/voquill/whisper-base.en-int8-ov/",
      "files": [
        {
          "name": "openvino_encoder_model.xml",
          "sha256": "replace-with-real-64-character-sha256"
        },
        {
          "name": "openvino_encoder_model.bin",
          "sha256": "replace-with-real-64-character-sha256"
        },
        {
          "name": "openvino_decoder_model.xml",
          "sha256": "replace-with-real-64-character-sha256"
        },
        {
          "name": "openvino_decoder_model.bin",
          "sha256": "replace-with-real-64-character-sha256"
        }
      ]
    }
  ]
}
```

This is deliberately not checked in as an active catalog because it is incomplete: real sidecars, real license URI, real hosting, and real hashes are required.

## Storage and Update Comparison

| Scenario | Current Voquill cache | Windows ML Model Catalog |
| --- | --- | --- |
| First install | Small app install; model downloaded later into `%APPDATA%\foss-voquill\models`; OpenVINO runtime separately under `%LOCALAPPDATA%\Voquill\openvino-runtime` | Small app install; model downloaded later into Windows ML user-specific shared location |
| Multiple app versions | Reuses same app-private path if model ids stay stable | Reuses catalog-managed files if same catalog identity/hash is requested |
| Multiple Windows users | Duplicated per user | Still user-specific per Microsoft docs, so duplicated across users |
| Multiple local-AI apps | Duplicated unless they manually share Voquill path | Shared only when apps request identical SHA256 model files through Model Catalog |
| Model upgrade | New id/path or overwrite behavior controlled by Voquill; no manifest GC today | Versioned catalog entries can coexist; cleanup behavior is more opaque |
| Rollback | App can keep old files if not deleted | Possible if old catalog version remains available, but needs explicit pinning |
| Uninstall | Tauri uninstall may leave app config/cache unless installer removes it; factory reset deletes models | Catalog files may remain in shared user cache; cleanup is less directly controlled by Voquill |
| Cache cleanup | App has direct delete via factory reset | App can stop referencing models but should not hardcode internal catalog paths |
| Integrity | GGML has SHA256 in metadata but current download code does not verify after download; OpenVINO entries have blank SHA256 | Catalog requires SHA256 for file entries and uses hashes for de-dupe |

Disk benefit for the current laptop would be limited unless another app uses the exact same model files through Model Catalog. Model Catalog would not reduce the 308.5 MB OpenVINO Python runtime, and it would not reduce runtime-private NPU compilation caches.

## Distribution, Licensing, UX, and Supply Chain

Adopting Model Catalog would require us to become a model catalog publisher or ship a local catalog source. For real use, that means:

- HTTPS hosting for catalog JSON and files.
- Complete SHA256 pinning for every file.
- Versioned model ids and rollback policy.
- License review for each model source. The current app points at Hugging Face repos; redistribution through our catalog/hosting may have different obligations than user-initiated download from the original repo.
- Explicit download consent, progress, cancellation, retry, metered-network behavior, and offline messaging.
- A pre-provisioning story for enterprise users if this becomes a managed deployment target.
- Tamper protection through hashes and controlled catalog updates.

The current UI already has user-initiated download progress. Model Catalog would improve hash discipline, but would add catalog hosting and Windows App SDK integration work.

## Performance Impact

No performance improvement is proven for the current OpenVINO GenAI path. Catalog retrieval is storage/download plumbing; it does not make OpenVINO GenAI faster by itself.

Migrating to Windows ML / ONNX Runtime could improve Windows integration and EP lifecycle, but it risks changing:

- Accuracy.
- Tokenizer behavior.
- Language hint behavior.
- Cold start and NPU compile time.
- Memory use.
- Warm model behavior.
- Intel NPU utilization.

Any ONNX/Windows ML path must be benchmarked against the current OpenVINO GenAI Base, Small, and Turbo measurements before it can replace the working NPU path.

## Required Changes for Adoption

If we adopt Model Catalog for ONNX variants only:

- Add a Windows-only model provider abstraction so `ModelManager` can resolve either app-cache paths or catalog-managed paths.
- Add a Windows ML/ONNX runtime path behind a feature flag or environment flag.
- Add Windows App SDK / Windows ML dependencies and packaging support.
- Add model catalog JSON generation/validation tooling.
- Host model files and catalog JSON over HTTPS, or ship a local catalog for controlled testing.
- Update Settings to show catalog-backed model status, download progress, offline failure, and managed-provider diagnostics.
- Keep current GGML/OpenVINO cache as fallback.

If we attempted to use Model Catalog for OpenVINO GenAI snapshots:

- Add catalog source definitions for every snapshot file and sidecar.
- Verify `ModelPaths` preserve a directory that `ov_genai.WhisperPipeline()` accepts.
- Keep OpenVINO Python runtime installation unchanged.
- Add fallback to existing downloader when catalog APIs are unavailable.
- Validate NPU performance and warmup behavior across clean install, restart, offline use, and model upgrade.

## Risks and Unknowns

- Rust/Tauri integration with Windows App SDK Model Catalog is not established in this repo.
- Catalog-managed path shape for multi-file models must be verified; we must not rely on undocumented internal cache paths.
- OpenVINO GenAI may require directory layout or auxiliary files beyond the XML/BIN files currently checked.
- OpenVINO GenAI NPU caches may be tied to runtime/driver versions and not portable.
- Model Catalog's de-dupe only helps for identical SHA256 files requested through the catalog.
- Current OpenVINO model metadata has blank SHA256 values; migration would require a stronger manifest.
- Current GGML `medium.en` hash appears copied from `small.en` in the registry and should be corrected before any stricter integrity work.

## Rollback Plan

Any future catalog spike should be opt-in and non-destructive:

1. Gate with `VOQUILL_USE_WINDOWS_ML_CATALOG=1`.
2. Keep current `%APPDATA%\foss-voquill\models` resolution as the default path.
3. Never delete app-cache models when testing catalog-backed resolution.
4. On catalog failure, log diagnostics and fall back to the existing `ModelManager` downloader.
5. Keep UI model ids stable; add a provider field rather than replacing existing model ids.
6. Remove the feature flag and dependency if performance or lifecycle behavior is not better.

## Go / No-Go Matrix

| Option | Storage win | NPU safety | Code/packaging cost | Recommendation |
| --- | --- | --- | --- | --- |
| Move all models to Model Catalog now | Medium only if shared with other catalog apps | Low | High | No-go |
| Use Model Catalog for current OpenVINO GenAI snapshots | Unknown | Medium until PoC proves path/perf | Medium-high | Spike later, not now |
| Add ONNX/Windows ML variants backed by Model Catalog | Potentially high for Windows integration | Unknown until benchmarked | High but cleanly isolated | Good larger spike |
| Keep current app/runtime-managed cache | No new sharing | High; preserves working NPU path | Low | Adopt now |
| Improve current cache integrity and cleanup | Medium operational win | High | Low-medium | Recommended next step |

## Recommendation

Defer Windows ML Model Catalog for the active Voquill model lifecycle. It is not incompatible as a file downloader, but it is only partially compatible with what Voquill actually runs today. The working Intel NPU path is OpenVINO GenAI, not Windows ML or ONNX Runtime, and a catalog migration would not by itself reduce the OpenVINO runtime footprint or prove any performance gain.

Next steps:

1. Keep the current OpenVINO GenAI NPU path fork-local and app-cache-backed.
2. Add SHA256 verification and atomic download staging to `ModelManager`, especially for OpenVINO snapshots.
3. Add model cache inventory/cleanup so old model variants do not orphan across upgrades.
4. Correct or remove suspect GGML hash metadata before relying on it.
5. Open a separate ONNX/Windows ML spike only if we want to test Windows ML's OpenVINO execution provider against the current OpenVINO GenAI performance.
6. Treat Model Catalog as a candidate for that ONNX spike, not as a drop-in replacement for Hugging Face/OpenVINO snapshot storage.
