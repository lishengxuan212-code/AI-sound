# Local Model Setup

The local ASR server is local-only. It does not use cloud ASR fallback and it must not contain real API keys.

## Default sherpa_onnx path

The default provider and model directory match the product requirement:

```powershell
$env:LOCAL_ASR_PROVIDER = "sherpa_onnx"
$env:LOCAL_ASR_MODEL_DIR = "local-asr/models/sherpa-onnx-streaming-paraformer-bilingual-zh-en"
```

`sherpa_onnx` is intentionally not wired to a guessed model constructor. Different sherpa-onnx streaming models require different config shapes, so the server probes local configuration and returns a clear local error if required files are missing.

Install dependency:

```powershell
pip install websockets sherpa-onnx
```

This project can also reuse the existing local ASR environment:

```text
E:\AI语音\.venv-local-asr\Scripts\python.exe
```

`scripts/start_local_asr.ps1` searches in this order:

1. project local `.venv-local-asr`
2. `E:\AI语音\.venv-local-asr`
3. system `python`

The server auto-detects these files from the default model directory:

- `tokens.txt`
- `encoder.int8.onnx` preferred, then `encoder.onnx`
- `decoder.int8.onnx` preferred, then `decoder.onnx`

You can override paths explicitly before starting:

```powershell
$env:LOCAL_SHERPA_ONNX_TOKENS = "local-asr/models/sherpa-onnx-streaming-paraformer-bilingual-zh-en/tokens.txt"
$env:LOCAL_SHERPA_ONNX_ENCODER = "local-asr/models/sherpa-onnx-streaming-paraformer-bilingual-zh-en/encoder.onnx"
$env:LOCAL_SHERPA_ONNX_DECODER = "local-asr/models/sherpa-onnx-streaming-paraformer-bilingual-zh-en/decoder.onnx"
.\scripts\start_local_asr.ps1
```

The bilingual streaming Paraformer model does not use a joiner file. If the server cannot construct the local recognizer, it reports that explicitly to the WebSocket client instead of silently falling back to any cloud ASR.

## Optional Vosk streaming path

1. Install the local dependencies:

```powershell
pip install websockets vosk
```

2. Download a Vosk model and extract it under `local-asr/models`.

Example local path:

```text
local-asr/models/vosk-model-small-cn-0.22
```

3. Start the local ASR WebSocket server:

```powershell
.\scripts\start_local_asr.ps1
```

To use Vosk explicitly:

```powershell
$env:LOCAL_ASR_MODEL_DIR = "local-asr/models/vosk-model-small-cn-0.22"
$env:LOCAL_ASR_PROVIDER = "vosk"
$env:LOCAL_ASR_SAMPLE_RATE = "16000"
$env:LOCAL_ASR_PORT = "8765"
.\scripts\start_local_asr.ps1
```

The server expects PCM 16-bit little-endian audio at `LOCAL_ASR_SAMPLE_RATE`. It emits JSON messages with `utteranceId`, `provider`, `text`, `isFinal`, `sessionKind`, and `sessionId`.

## App configuration

Configure local translation in `.env` using `.env.example` as the non-secret template.

No API keys should be committed, printed, or placed in frontend stores.
