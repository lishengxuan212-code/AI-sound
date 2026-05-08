import argparse
import json
import logging
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any


LOGGER = logging.getLogger("local-translation")


class LocalTranslationEngine:
    def __init__(self, model_path: str, model_name: str) -> None:
        self.model_path = model_path
        self.model_name = model_name
        self._tokenizer: Any | None = None
        self._model: Any | None = None
        self._load_error: str | None = None
        self._load_model()

    def _load_model(self) -> None:
        configured = self.model_path or self.model_name
        if not configured:
            self._load_error = "本地翻译模型未配置或未启动"
            LOGGER.warning(self._load_error)
            return

        model_ref = self.model_path or self.model_name
        if self.model_path and not Path(self.model_path).exists():
            self._load_error = f"本地翻译模型未配置或未启动: {self.model_path}"
            LOGGER.error(self._load_error)
            return

        try:
            from transformers import AutoModelForSeq2SeqLM, AutoTokenizer  # type: ignore
        except ImportError:
            self._load_error = "本地翻译模型未配置或未启动: transformers 未安装"
            LOGGER.error(self._load_error)
            return

        try:
            self._tokenizer = AutoTokenizer.from_pretrained(model_ref, local_files_only=bool(self.model_path))
            self._model = AutoModelForSeq2SeqLM.from_pretrained(model_ref, local_files_only=bool(self.model_path))
            LOGGER.info("Loaded local translation model: %s", model_ref)
        except Exception as exc:
            self._load_error = f"本地翻译模型未配置或未启动: {exc}"
            LOGGER.exception(self._load_error)

    def translate(self, source_text: str) -> tuple[str, str | None]:
        if not self._tokenizer or not self._model:
            return "", self._load_error or "本地翻译模型未配置或未启动"
        if not source_text.strip():
            return "", None

        inputs = self._tokenizer(source_text, return_tensors="pt", truncation=True)
        output_ids = self._model.generate(**inputs, max_new_tokens=int(os.getenv("LOCAL_TRANSLATION_MAX_NEW_TOKENS", "256")))
        translated = self._tokenizer.decode(output_ids[0], skip_special_tokens=True)
        return translated.strip(), None


class TranslationHandler(BaseHTTPRequestHandler):
    engine: LocalTranslationEngine

    def do_POST(self) -> None:
        if self.path.rstrip("/") != "/translate":
            self._send_json(404, {"status": "failed", "error": "not found"})
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
            payload = json.loads(self.rfile.read(length).decode("utf-8"))
        except Exception as exc:
            self._send_json(400, {"status": "failed", "error": f"invalid request: {exc}"})
            return

        request_id = str(payload.get("requestId", ""))
        source_text = str(payload.get("sourceText", ""))
        translated, error = self.engine.translate(source_text)
        if error:
            self._send_json(
                503,
                {
                    "requestId": request_id,
                    "translatedText": "",
                    "status": "failed",
                    "error": error,
                },
            )
            return

        self._send_json(
            200,
            {
                "requestId": request_id,
                "translatedText": translated,
                "status": "completed",
                "error": None,
            },
        )

    def do_GET(self) -> None:
        if self.path.rstrip("/") == "/health":
            self._send_json(200, {"status": "ok", "modelLoaded": self.engine._model is not None})
            return
        self._send_json(404, {"status": "failed", "error": "not found"})

    def log_message(self, format: str, *args: Any) -> None:
        LOGGER.info(format, *args)

    def _send_json(self, status: int, payload: dict[str, Any]) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default=os.getenv("LOCAL_TRANSLATION_HOST", "127.0.0.1"))
    parser.add_argument("--port", type=int, default=int(os.getenv("LOCAL_TRANSLATION_PORT", "8770")))
    parser.add_argument("--model-path", default=os.getenv("LOCAL_TRANSLATION_MODEL_PATH", ""))
    parser.add_argument("--model-name", default=os.getenv("LOCAL_TRANSLATION_MODEL_NAME", ""))
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="[LOCAL_TRANSLATION] %(message)s")
    TranslationHandler.engine = LocalTranslationEngine(args.model_path, args.model_name)
    server = ThreadingHTTPServer((args.host, args.port), TranslationHandler)
    LOGGER.info("Starting local translation server at http://%s:%s/translate", args.host, args.port)
    LOGGER.info("No cloud translation fallback is configured")
    server.serve_forever()


if __name__ == "__main__":
    main()
