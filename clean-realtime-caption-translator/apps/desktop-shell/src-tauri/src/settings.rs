use serde::Serialize;
use std::env;

#[derive(Debug, Serialize)]
pub struct LocalAsrSettings {
    pub provider: String,
    #[serde(rename = "wsUrl")]
    pub ws_url: String,
    #[serde(rename = "modelDir")]
    pub model_dir: String,
    #[serde(rename = "sourceLang")]
    pub source_lang: String,
}

#[derive(Debug, Serialize)]
pub struct LocalTranslationSettings {
    pub provider: String,
    pub endpoint: String,
    #[serde(rename = "modelPath")]
    pub model_path: String,
    #[serde(rename = "modelName")]
    pub model_name: String,
    #[serde(rename = "sourceLang")]
    pub source_lang: String,
    #[serde(rename = "targetLang")]
    pub target_lang: String,
}

#[derive(Debug, Serialize)]
pub struct QwenTtsSettings {
    pub endpoint: String,
    pub model: String,
    pub voice: String,
    pub format: String,
    #[serde(rename = "sampleRate")]
    pub sample_rate: u32,
    pub speed: f32,
    pub volume: u32,
    #[serde(rename = "hasApiKey")]
    pub has_api_key: bool,
}

#[derive(Debug, Serialize)]
pub struct AppSettings {
    #[serde(rename = "localAsr")]
    pub local_asr: LocalAsrSettings,
    #[serde(rename = "localTranslation")]
    pub local_translation: LocalTranslationSettings,
    #[serde(rename = "qwenTts")]
    pub qwen_tts: QwenTtsSettings,
    #[serde(rename = "logLevel")]
    pub log_level: String,
    #[serde(rename = "diagnosticsEnabled")]
    pub diagnostics_enabled: bool,
}

fn env_or(name: &str, fallback: &str) -> String {
    env::var(name).unwrap_or_else(|_| fallback.to_string())
}

pub fn load_app_settings() -> AppSettings {
    let has_api_key = env::var("GAME_TTS_API_KEY").map(|value| !value.is_empty()).unwrap_or(false)
        || env::var("DASHSCOPE_API_KEY").map(|value| !value.is_empty()).unwrap_or(false);

    AppSettings {
        local_asr: LocalAsrSettings {
            provider: env_or("LOCAL_ASR_PROVIDER", "sherpa_onnx"),
            ws_url: env_or("LOCAL_ASR_WS_URL", "ws://127.0.0.1:8765/ws"),
            model_dir: env_or(
                "LOCAL_ASR_MODEL_DIR",
                "local-asr/models/sherpa-onnx-streaming-paraformer-bilingual-zh-en",
            ),
            source_lang: env_or("LOCAL_TRANSLATION_SOURCE_LANG", "en"),
        },
        local_translation: LocalTranslationSettings {
            provider: env_or("LOCAL_TRANSLATION_PROVIDER", ""),
            endpoint: env_or("LOCAL_TRANSLATION_ENDPOINT", ""),
            model_path: env_or("LOCAL_TRANSLATION_MODEL_PATH", ""),
            model_name: env_or("LOCAL_TRANSLATION_MODEL_NAME", ""),
            source_lang: env_or("LOCAL_TRANSLATION_SOURCE_LANG", "en"),
            target_lang: env_or("LOCAL_TRANSLATION_TARGET_LANG", "zh"),
        },
        qwen_tts: QwenTtsSettings {
            endpoint: env_or(
                "GAME_TTS_API_ENDPOINT",
                "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation",
            ),
            model: env_or("GAME_TTS_API_MODEL", "qwen-qwen-tts-latest"),
            voice: env_or("GAME_TTS_API_VOICE", ""),
            format: env_or("GAME_TTS_API_FORMAT", "wav"),
            sample_rate: env_or("GAME_TTS_API_SAMPLE_RATE", "24000").parse().unwrap_or(24000),
            speed: env_or("GAME_TTS_API_SPEED", "1.0").parse().unwrap_or(1.0),
            volume: env_or("GAME_TTS_API_VOLUME", "50").parse().unwrap_or(50),
            has_api_key,
        },
        log_level: env_or("APP_LOG_LEVEL", "info"),
        diagnostics_enabled: env_or("APP_ENABLE_DIAGNOSTICS", "true") == "true",
    }
}

pub fn tts_api_key() -> Option<String> {
    env::var("GAME_TTS_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| env::var("DASHSCOPE_API_KEY").ok().filter(|value| !value.is_empty()))
}
