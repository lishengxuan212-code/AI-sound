import argparse
import asyncio
import json
import logging
import os
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any

try:
    import websockets
except ImportError as exc:  # pragma: no cover
    raise SystemExit("Install websockets before starting local ASR: pip install websockets") from exc


LOGGER = logging.getLogger("local-asr")
PROJECT_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EN_MODEL_DIR = PROJECT_ROOT / "local-asr" / "models" / "vosk-model-small-en-us-0.15"
DEFAULT_ZH_MODEL_DIR = PROJECT_ROOT / "local-asr" / "models" / "sherpa-onnx-streaming-paraformer-bilingual-zh-en"


@dataclass
class AsrConfig:
    provider: str
    model_dir: str
    en_model_dir: str
    zh_model_dir: str
    host: str
    port: int
    sample_rate: float


class LocalAsrEngine:
    def __init__(self, config: AsrConfig) -> None:
        self.config = config
        self.engine_name = f"local-{config.provider}-realtime"
        self._engine: Any | None = None
        self._model: Any | None = None
        self._vosk_engine: Any | None = None
        self._vosk_models: dict[str, Any] = {}
        self._vosk_model_errors: dict[str, str] = {}
        self._sherpa_engine: Any | None = None
        self._sherpa_model: dict[str, str] | None = None
        self._sherpa_error: str | None = None
        self._load_error: str | None = None
        self._load_engine()

    def _load_engine(self) -> None:
        provider = self.config.provider
        if provider == "vosk":
            self.engine_name = "local-vosk-sherpa-realtime"
            self._load_vosk()
            self._load_sherpa_zh()
            return

        if provider == "sherpa_onnx":
            self.engine_name = "local-sherpa-onnx-realtime"
            self._probe_sherpa_onnx()
            return

        self._load_error = (
            f"Unsupported LOCAL_ASR_PROVIDER={provider!r}. "
            "Supported providers: vosk, sherpa_onnx."
        )
        LOGGER.warning(self._load_error)

    def _load_vosk(self) -> None:
        model_dirs = {
            "default": self.config.model_dir,
            "en": self.config.en_model_dir or self.config.model_dir,
        }
        loaded_any = False
        for lang, model_dir in model_dirs.items():
            model_path = Path(model_dir)
            model = self._load_vosk_model(lang, model_path)
            if model is not None:
                self._vosk_models[lang] = model
                loaded_any = True
        if loaded_any:
            self._model = self._vosk_models.get("en") or self._vosk_models.get("default") or next(iter(self._vosk_models.values()))
            return
        self._load_error = "; ".join(self._vosk_model_errors.values()) or "No Vosk model is available"

    def _load_vosk_model(self, lang: str, model_path: Path) -> Any | None:
        if not model_path.exists():
            error = f"Vosk model directory does not exist for {lang}: {model_path}"
            self._vosk_model_errors[lang] = error
            LOGGER.warning(error)
            return None

        try:
            import vosk  # type: ignore
        except ImportError:
            error = "vosk is not installed. Install it with: pip install vosk"
            self._vosk_model_errors[lang] = error
            LOGGER.warning(error)
            return None

        try:
            vosk.SetLogLevel(-1)
            model = vosk.Model(str(model_path))
            self._vosk_engine = vosk
            if self._engine is None:
                self._engine = vosk
            LOGGER.info(
                "Loaded Vosk %s model from %s with sample_rate=%s",
                lang,
                model_path,
                self.config.sample_rate,
            )
            return model
        except Exception as exc:  # pragma: no cover - depends on native model files
            error = f"Failed to load Vosk model for {lang} from {model_path}: {exc}"
            self._vosk_model_errors[lang] = error
            LOGGER.exception(error)
            return None

    def _probe_sherpa_onnx(self) -> None:
        loaded = self._load_sherpa_model(self.config.model_dir, "default")
        if loaded is None:
            self._load_error = self._sherpa_error
            return
        self._sherpa_engine, self._sherpa_model = loaded
        self._engine = self._sherpa_engine
        self._model = self._sherpa_model

    def _load_sherpa_zh(self) -> None:
        loaded = self._load_sherpa_model(self.config.zh_model_dir, "zh")
        if loaded is None:
            LOGGER.warning("Chinese sherpa_onnx model unavailable: %s", self._sherpa_error)
            return
        self._sherpa_engine, self._sherpa_model = loaded
        if self._engine is None:
            self._engine = self._sherpa_engine
        LOGGER.info("Loaded sherpa_onnx zh model from %s", self.config.zh_model_dir)

    def _load_sherpa_model(self, model_dir: str, label: str) -> tuple[Any, dict[str, str]] | None:
        try:
            import sherpa_onnx  # type: ignore
        except ImportError:
            self._sherpa_error = "sherpa_onnx is not installed. Install it with: pip install sherpa-onnx"
            LOGGER.warning(self._sherpa_error)
            return None

        model_path = Path(model_dir)
        if not model_path.exists():
            self._sherpa_error = f"sherpa_onnx {label} model directory does not exist: {model_path}"
            LOGGER.warning(self._sherpa_error)
            return None

        tokens = self._sherpa_path(model_path, "LOCAL_SHERPA_ONNX_TOKENS", "tokens.txt")
        encoder = self._sherpa_path(model_path, "LOCAL_SHERPA_ONNX_ENCODER", "encoder.int8.onnx", "encoder.onnx")
        decoder = self._sherpa_path(model_path, "LOCAL_SHERPA_ONNX_DECODER", "decoder.int8.onnx", "decoder.onnx")
        missing = [str(path) for path in [tokens, encoder, decoder] if not path.is_file()]
        if missing:
            detected = sorted(item.name for item in model_path.glob("*") if item.is_file())
            self._sherpa_error = (
                "sherpa_onnx is installed, but required streaming Paraformer files are missing: "
                f"{', '.join(missing)}. "
                f"Detected files in {model_path}: {', '.join(detected) or '(none)'}."
            )
            LOGGER.error(self._sherpa_error)
            return None

        model = {
            "tokens": str(tokens),
            "encoder": str(encoder),
            "decoder": str(decoder),
        }
        LOGGER.info("Configured sherpa_onnx Paraformer with encoder=%s decoder=%s", encoder, decoder)
        return sherpa_onnx, model

    def _sherpa_path(self, model_path: Path, env_name: str, *fallback_names: str) -> Path:
        explicit = os.getenv(env_name)
        if explicit:
            return Path(explicit)
        for name in fallback_names:
            candidate = model_path / name
            if candidate.is_file():
                return candidate
        return model_path / fallback_names[0]

    def create_session(self, utterance_id: str, sample_rate: float | None = None, language: str | None = None) -> "LocalAsrSession":
        lang = normalize_lang(language or "")
        if lang == "zh" and self._sherpa_engine and self._sherpa_model:
            return SherpaParaformerAsrSession(
                self,
                utterance_id,
                sample_rate or self.config.sample_rate,
                self._sherpa_engine,
                self._sherpa_model,
            )
        if self.config.provider == "vosk" and self._vosk_engine and self._model:
            return VoskAsrSession(self, utterance_id, sample_rate or self.config.sample_rate, self.vosk_model_for(language))
        if self.config.provider == "sherpa_onnx" and self._sherpa_engine and self._sherpa_model:
            return SherpaParaformerAsrSession(
                self,
                utterance_id,
                sample_rate or self.config.sample_rate,
                self._sherpa_engine,
                self._sherpa_model,
            )
        return ErrorAsrSession(self, utterance_id)

    def vosk_model_for(self, language: str | None) -> Any:
        lang = normalize_lang(language or "")
        return self._vosk_models.get(lang) or self._vosk_models.get("default") or self._model

    def health_result(self) -> dict[str, Any]:
        return {
            "type": "health",
            "ok": self._engine is not None and self._model is not None,
            "provider": self.engine_name,
            "modelDir": self.config.model_dir,
            "modelDirs": {
                "en": self.config.en_model_dir,
                "zh": self.config.zh_model_dir,
            },
            "loadedLanguages": sorted(self._vosk_models.keys()),
            "loadedEngines": {
                "en": "vosk" if self._vosk_models.get("en") else None,
                "zh": "sherpa_onnx" if self._sherpa_model else ("vosk" if self._vosk_models.get("zh") else None),
            },
            "error": self._load_error,
        }

    def error_result(self, utterance_id: str, message: str | None = None) -> dict[str, Any]:
        return {
            "utteranceId": utterance_id,
            "provider": self.engine_name,
            "text": "",
            "isFinal": False,
            "error": message or self._load_error or "Local ASR engine is not configured",
        }


class LocalAsrSession:
    def __init__(self, engine: LocalAsrEngine, utterance_id: str) -> None:
        self.engine = engine
        self.utterance_id = utterance_id

    async def accept_audio(self, pcm: bytes) -> dict[str, Any] | None:
        raise NotImplementedError

    async def finish(self) -> dict[str, Any] | None:
        return None


class ErrorAsrSession(LocalAsrSession):
    def __init__(self, engine: LocalAsrEngine, utterance_id: str) -> None:
        super().__init__(engine, utterance_id)
        self._sent = False

    async def accept_audio(self, pcm: bytes) -> dict[str, Any] | None:
        if self._sent:
            return None
        self._sent = True
        return self.engine.error_result(self.utterance_id)


class VoskAsrSession(LocalAsrSession):
    def __init__(self, engine: LocalAsrEngine, utterance_id: str, sample_rate: float, model: Any) -> None:
        super().__init__(engine, utterance_id)
        self._recognizer = engine._vosk_engine.KaldiRecognizer(model, sample_rate)
        self._recognizer.SetWords(True)
        self._last_partial = ""

    async def accept_audio(self, pcm: bytes) -> dict[str, Any] | None:
        if not pcm:
            return None

        is_final = bool(self._recognizer.AcceptWaveform(pcm))
        payload = self._read_result(is_final)
        text = self._extract_text(payload, is_final)

        if not is_final and text == self._last_partial:
            return None
        if not is_final:
            self._last_partial = text
        elif text:
            self._last_partial = ""

        if not text:
            return None
        return self._format_result(text, is_final, payload)

    async def finish(self) -> dict[str, Any] | None:
        payload = json.loads(self._recognizer.FinalResult())
        text = payload.get("text", "").strip()
        if not text:
            return None
        return self._format_result(text, True, payload)

    def _read_result(self, is_final: bool) -> dict[str, Any]:
        raw = self._recognizer.Result() if is_final else self._recognizer.PartialResult()
        return json.loads(raw)

    @staticmethod
    def _extract_text(payload: dict[str, Any], is_final: bool) -> str:
        key = "text" if is_final else "partial"
        return str(payload.get(key, "")).strip()

    def _format_result(self, text: str, is_final: bool, payload: dict[str, Any]) -> dict[str, Any]:
        result: dict[str, Any] = {
            "utteranceId": self.utterance_id,
            "provider": self.engine.engine_name,
            "text": text,
            "isFinal": is_final,
        }
        word_items = payload.get("result")
        if isinstance(word_items, list) and word_items:
            start = word_items[0].get("start")
            end = word_items[-1].get("end")
            if isinstance(start, (int, float)):
                result["sourceStartMs"] = int(start * 1000)
            if isinstance(end, (int, float)):
                result["sourceEndMs"] = int(end * 1000)
        return result


class SherpaParaformerAsrSession(LocalAsrSession):
    def __init__(self, engine: LocalAsrEngine, utterance_id: str, sample_rate: float, sherpa_engine: Any, model: dict[str, str]) -> None:
        super().__init__(engine, utterance_id)
        try:
            import numpy as np  # type: ignore
        except ImportError as exc:
            raise RuntimeError("numpy is required for sherpa_onnx streaming audio input") from exc

        self._np = np
        self._sample_rate = int(sample_rate)
        self._recognizer = sherpa_engine.OnlineRecognizer.from_paraformer(
            tokens=model["tokens"],
            encoder=model["encoder"],
            decoder=model["decoder"],
            num_threads=int(os.getenv("LOCAL_SHERPA_ONNX_NUM_THREADS", "1")),
            sample_rate=16000,
            feature_dim=80,
            enable_endpoint_detection=True,
            rule1_min_trailing_silence=float(os.getenv("LOCAL_SHERPA_ONNX_RULE1_SILENCE", "2.4")),
            rule2_min_trailing_silence=float(os.getenv("LOCAL_SHERPA_ONNX_RULE2_SILENCE", "1.2")),
            rule3_min_utterance_length=float(os.getenv("LOCAL_SHERPA_ONNX_RULE3_MIN_UTTERANCE", "300")),
        )
        self._stream = self._recognizer.create_stream()
        self._last_partial = ""

    async def accept_audio(self, pcm: bytes) -> dict[str, Any] | None:
        if not pcm:
            return None
        samples = self._pcm16le_to_float32(pcm)
        self._stream.accept_waveform(self._sample_rate, samples)
        while self._recognizer.is_ready(self._stream):
            self._recognizer.decode_stream(self._stream)

        text = str(self._recognizer.get_result(self._stream)).strip()
        is_endpoint = bool(self._recognizer.is_endpoint(self._stream))
        if is_endpoint:
            result = self._format_result(text, True)
            self._recognizer.reset(self._stream)
            self._last_partial = ""
            return result

        if text and text != self._last_partial:
            self._last_partial = text
            return self._format_result(text, False)
        return None

    async def finish(self) -> dict[str, Any] | None:
        text = str(self._recognizer.get_result(self._stream)).strip()
        if not text:
            return None
        return self._format_result(text, True)

    def _pcm16le_to_float32(self, pcm: bytes) -> Any:
        samples = self._np.frombuffer(pcm, dtype=self._np.int16).astype(self._np.float32)
        return samples / 32768.0

    def _format_result(self, text: str, is_final: bool) -> dict[str, Any] | None:
        if not text:
            return None
        return {
            "utteranceId": self.utterance_id,
            "provider": self.engine.engine_name,
            "text": text,
            "isFinal": is_final,
        }


def attach_session_fields(result: dict[str, Any], session_kind: str, session_id: str) -> dict[str, Any]:
    result["sessionKind"] = session_kind
    result["sessionId"] = session_id
    return result


def normalize_lang(lang: str) -> str:
    value = (lang or "").strip().lower().replace("_", "-")
    if value in {"zh", "zh-cn", "zh-hans", "chinese"}:
        return "zh"
    if value in {"en", "en-us", "english"}:
        return "en"
    return value or "default"


async def handle_connection(websocket: Any, engine: LocalAsrEngine) -> None:
    utterance_id = str(uuid.uuid4())
    session_kind = "SystemSubtitle"
    session_id = ""
    language = "default"
    asr_session = engine.create_session(utterance_id, language=language)
    LOGGER.info("Local ASR client connected")

    try:
        async for message in websocket:
            if isinstance(message, str):
                payload = json.loads(message)
                if payload.get("type") == "health":
                    await websocket.send(json.dumps(engine.health_result(), ensure_ascii=False))
                    continue
                session_kind = payload.get("sessionKind", session_kind)
                session_id = payload.get("sessionId", session_id)
                language = payload.get("language", language)
                if payload.get("type") == "start":
                    sample_rate = float(payload.get("sampleRate") or engine.config.sample_rate)
                    utterance_id = str(uuid.uuid4())
                    asr_session = engine.create_session(utterance_id, sample_rate, language)
                if payload.get("prompt"):
                    LOGGER.info("prompt received but may be ignored by local ASR engine")
                continue

            result = await asr_session.accept_audio(message)
            if result:
                await websocket.send(
                    json.dumps(attach_session_fields(result, session_kind, session_id), ensure_ascii=False)
                )
    finally:
        result = await asr_session.finish()
        if result:
            try:
                await websocket.send(
                    json.dumps(attach_session_fields(result, session_kind, session_id), ensure_ascii=False)
                )
            except websockets.exceptions.ConnectionClosed:
                LOGGER.debug("Client disconnected before final ASR result could be sent")


async def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default=os.getenv("LOCAL_ASR_HOST", "127.0.0.1"))
    parser.add_argument("--port", type=int, default=int(os.getenv("LOCAL_ASR_PORT", "8765")))
    parser.add_argument("--provider", default=os.getenv("LOCAL_ASR_PROVIDER", "vosk"))
    parser.add_argument(
        "--model-dir",
        default=os.getenv(
            "LOCAL_ASR_MODEL_DIR",
            str(DEFAULT_EN_MODEL_DIR),
        ),
    )
    parser.add_argument(
        "--en-model-dir",
        default=os.getenv("LOCAL_ASR_MODEL_DIR_EN", str(DEFAULT_EN_MODEL_DIR)),
    )
    parser.add_argument(
        "--zh-model-dir",
        default=os.getenv("LOCAL_ASR_MODEL_DIR_ZH", str(DEFAULT_ZH_MODEL_DIR)),
    )
    parser.add_argument("--sample-rate", type=float, default=float(os.getenv("LOCAL_ASR_SAMPLE_RATE", "16000")))
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="[LOCAL_ASR] %(message)s")
    config = AsrConfig(
        provider=args.provider,
        model_dir=args.model_dir,
        en_model_dir=args.en_model_dir,
        zh_model_dir=args.zh_model_dir,
        host=args.host,
        port=args.port,
        sample_rate=args.sample_rate,
    )
    engine = LocalAsrEngine(config)

    LOGGER.info("Starting local ASR WebSocket server at ws://%s:%s/ws", config.host, config.port)
    LOGGER.info("Provider: %s", config.provider)
    LOGGER.info("Model directory: %s", config.model_dir)
    LOGGER.info("English model directory: %s", config.en_model_dir)
    LOGGER.info("Chinese model directory: %s", config.zh_model_dir)
    LOGGER.info("Sample rate: %s Hz PCM 16-bit little-endian mono expected", config.sample_rate)
    LOGGER.info("No cloud ASR fallback is configured")
    async with websockets.serve(lambda ws: handle_connection(ws, engine), config.host, config.port):
        await asyncio.Future()


if __name__ == "__main__":
    asyncio.run(main())
