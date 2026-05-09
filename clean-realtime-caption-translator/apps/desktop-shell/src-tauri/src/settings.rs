use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalAsrSettings {
    pub provider: String,
    #[serde(rename = "wsUrl")]
    pub ws_url: String,
    #[serde(rename = "modelDir")]
    pub model_dir: String,
    #[serde(rename = "sampleRate")]
    pub sample_rate: u32,
    #[serde(rename = "languageMode")]
    pub language_mode: String,
    #[serde(rename = "enableEndpointDetection")]
    pub enable_endpoint_detection: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalTranslationSettings {
    pub provider: String,
    pub endpoint: String,
    #[serde(rename = "apiKey", default)]
    pub api_key: String,
    #[serde(rename = "proxyUrl", default)]
    pub proxy_url: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(rename = "modelPath")]
    pub model_path: String,
    #[serde(rename = "modelName")]
    pub model_name: String,
    #[serde(rename = "sourceLang")]
    pub source_lang: String,
    #[serde(rename = "targetLang")]
    pub target_lang: String,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QwenTtsSettings {
    pub endpoint: String,
    #[serde(rename = "apiKey", default)]
    pub api_key: String,
    pub model: String,
    pub voice: String,
    pub format: String,
    #[serde(rename = "sampleRate")]
    pub sample_rate: u32,
    pub speed: f32,
    pub volume: u32,
    #[serde(rename = "hasApiKey")]
    pub has_api_key: bool,
    #[serde(rename = "gameTtsApiKeyConfigured")]
    pub game_tts_api_key_configured: bool,
    #[serde(rename = "dashscopeApiKeyConfigured")]
    pub dashscope_api_key_configured: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioDeviceSettings {
    #[serde(rename = "systemOutputDeviceName")]
    pub system_output_device_name: String,
    #[serde(rename = "micInputDeviceName")]
    pub mic_input_device_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LanguageSettings {
    #[serde(rename = "systemSourceLang")]
    pub system_source_lang: String,
    #[serde(rename = "systemTargetLang")]
    pub system_target_lang: String,
    #[serde(rename = "micSourceLang")]
    pub mic_source_lang: String,
    #[serde(rename = "micTargetLang")]
    pub mic_target_lang: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemSegmenterSettings {
    #[serde(rename = "minWordsBeforeFinalize")]
    pub min_words_before_finalize: u32,
    #[serde(rename = "preferredMaxChars")]
    pub preferred_max_chars: u32,
    #[serde(rename = "hardMaxChars")]
    pub hard_max_chars: u32,
    #[serde(rename = "silenceForVisualFinalizeMs")]
    pub silence_for_visual_finalize_ms: u32,
    #[serde(rename = "allowCommaFinalize")]
    pub allow_comma_finalize: bool,
    #[serde(rename = "allowShortPauseFinalize")]
    pub allow_short_pause_finalize: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MicSegmenterSettings {
    #[serde(rename = "minWordsBeforeTranslate")]
    pub min_words_before_translate: u32,
    #[serde(rename = "preferredMaxChars")]
    pub preferred_max_chars: u32,
    #[serde(rename = "hardMaxChars")]
    pub hard_max_chars: u32,
    #[serde(rename = "silenceForTranslateMs")]
    pub silence_for_translate_ms: u32,
    #[serde(rename = "allowCommaTranslate")]
    pub allow_comma_translate: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DiagnosticsSettings {
    #[serde(rename = "logLevel")]
    pub log_level: String,
    #[serde(rename = "diagnosticsEnabled")]
    pub diagnostics_enabled: bool,
    #[serde(rename = "showSystemLogs")]
    pub show_system_logs: bool,
    #[serde(rename = "showMicLogs")]
    pub show_mic_logs: bool,
    #[serde(rename = "showAsrLogs")]
    pub show_asr_logs: bool,
    #[serde(rename = "showTranslationLogs")]
    pub show_translation_logs: bool,
    #[serde(rename = "showTtsLogs")]
    pub show_tts_logs: bool,
    #[serde(rename = "showErrorLogs")]
    pub show_error_logs: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppSettings {
    #[serde(rename = "localAsr")]
    pub local_asr: LocalAsrSettings,
    #[serde(rename = "localTranslation")]
    pub local_translation: LocalTranslationSettings,
    #[serde(rename = "qwenTts")]
    pub qwen_tts: QwenTtsSettings,
    #[serde(rename = "audioDevices", default = "default_audio_devices")]
    pub audio_devices: AudioDeviceSettings,
    pub language: LanguageSettings,
    #[serde(rename = "systemSegmenter")]
    pub system_segmenter: SystemSegmenterSettings,
    #[serde(rename = "micSegmenter")]
    pub mic_segmenter: MicSegmenterSettings,
    pub diagnostics: DiagnosticsSettings,
    #[serde(rename = "logLevel")]
    pub log_level: String,
    #[serde(rename = "diagnosticsEnabled")]
    pub diagnostics_enabled: bool,
    #[serde(rename = "e2eAutostartSystemSubtitle")]
    pub e2e_autostart_system_subtitle: bool,
}

#[derive(Debug, Serialize)]
pub struct SecretStatus {
    #[serde(rename = "gameTtsApiKeyConfigured")]
    pub game_tts_api_key_configured: bool,
    #[serde(rename = "dashscopeApiKeyConfigured")]
    pub dashscope_api_key_configured: bool,
}

#[derive(Debug, Serialize)]
pub struct ConnectionTestResult {
    pub ok: bool,
    pub message: String,
}

fn env_or(name: &str, fallback: &str) -> String {
    load_dotenv_once();
    env::var(name).unwrap_or_else(|_| fallback.to_string())
}

fn env_bool(name: &str, fallback: bool) -> bool {
    env_or(name, if fallback { "true" } else { "false" }) == "true"
}

fn default_audio_devices() -> AudioDeviceSettings {
    AudioDeviceSettings {
        system_output_device_name: env_or("SYSTEM_AUDIO_OUTPUT_DEVICE", ""),
        mic_input_device_name: env_or("MIC_AUDIO_INPUT_DEVICE", ""),
    }
}

fn load_dotenv_once() {
    static DOTENV: OnceLock<()> = OnceLock::new();
    DOTENV.get_or_init(|| {
        let _ = dotenvy::dotenv();
    });
}

fn settings_path() -> Result<PathBuf, String> {
    let current = std::env::current_dir().map_err(|error| error.to_string())?;
    let app_dir = if current
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "src-tauri")
    {
        current
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| current.clone())
    } else {
        current
    };
    Ok(app_dir.join("app-settings.json"))
}

fn legacy_settings_path() -> Result<PathBuf, String> {
    std::env::current_dir()
        .map_err(|error| error.to_string())
        .map(|path| path.join("app-settings.json"))
}

pub fn default_app_settings() -> AppSettings {
    let secret_status = get_secret_status();
    let has_api_key =
        secret_status.game_tts_api_key_configured || secret_status.dashscope_api_key_configured;
    let log_level = env_or("APP_LOG_LEVEL", "info");
    let diagnostics_enabled = env_bool("APP_ENABLE_DIAGNOSTICS", false);

    AppSettings {
        local_asr: LocalAsrSettings {
            provider: env_or("LOCAL_ASR_PROVIDER", "vosk"),
            ws_url: env_or("LOCAL_ASR_WS_URL", "ws://127.0.0.1:8765/ws"),
            model_dir: env_or(
                "LOCAL_ASR_MODEL_DIR",
                r"local-asr\models\vosk-model-small-en-us-0.15",
            ),
            sample_rate: env_or("LOCAL_ASR_SAMPLE_RATE", "16000")
                .parse()
                .unwrap_or(16000),
            language_mode: env_or("LOCAL_ASR_LANGUAGE_MODE", "bilingual"),
            enable_endpoint_detection: env_bool("LOCAL_ASR_ENABLE_ENDPOINT_DETECTION", true),
        },
        local_translation: LocalTranslationSettings {
            provider: env_or("LOCAL_TRANSLATION_PROVIDER", "dashscope_openai"),
            endpoint: env_or(
                "LOCAL_TRANSLATION_ENDPOINT",
                "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions",
            ),
            api_key: env::var("LOCAL_TRANSLATION_API_KEY")
                .ok()
                .filter(|value| !value.is_empty())
                .or_else(|| {
                    env::var("DASHSCOPE_API_KEY")
                        .ok()
                        .filter(|value| !value.is_empty())
                })
                .unwrap_or_default(),
            proxy_url: env_or("LOCAL_TRANSLATION_PROXY_URL", ""),
            prompt: env_or("LOCAL_TRANSLATION_PROMPT", ""),
            model_path: env_or("LOCAL_TRANSLATION_MODEL_PATH", ""),
            model_name: env_or("LOCAL_TRANSLATION_MODEL_NAME", ""),
            source_lang: env_or("LOCAL_TRANSLATION_SOURCE_LANG", "en"),
            target_lang: env_or("LOCAL_TRANSLATION_TARGET_LANG", "zh"),
            timeout_ms: env_or("LOCAL_TRANSLATION_TIMEOUT_MS", "15000")
                .parse()
                .unwrap_or(15000),
        },
        qwen_tts: QwenTtsSettings {
            endpoint: env_or(
                "GAME_TTS_API_ENDPOINT",
                "https://dashscope.aliyuncs.com/api/v1/services/audio/tts/SpeechSynthesizer",
            ),
            api_key: env::var("GAME_TTS_API_KEY")
                .ok()
                .filter(|value| !value.is_empty())
                .or_else(|| {
                    env::var("DASHSCOPE_API_KEY")
                        .ok()
                        .filter(|value| !value.is_empty())
                })
                .unwrap_or_default(),
            model: env_or("GAME_TTS_API_MODEL", "qwen-qwen-tts-latest"),
            voice: env_or("GAME_TTS_API_VOICE", ""),
            format: env_or("GAME_TTS_API_FORMAT", "wav"),
            sample_rate: env_or("GAME_TTS_API_SAMPLE_RATE", "24000")
                .parse()
                .unwrap_or(24000),
            speed: env_or("GAME_TTS_API_SPEED", "1.0").parse().unwrap_or(1.0),
            volume: env_or("GAME_TTS_API_VOLUME", "50").parse().unwrap_or(50),
            has_api_key,
            game_tts_api_key_configured: secret_status.game_tts_api_key_configured,
            dashscope_api_key_configured: secret_status.dashscope_api_key_configured,
        },
        audio_devices: AudioDeviceSettings {
            system_output_device_name: env_or("SYSTEM_AUDIO_OUTPUT_DEVICE", ""),
            mic_input_device_name: env_or("MIC_AUDIO_INPUT_DEVICE", ""),
        },
        language: LanguageSettings {
            system_source_lang: env_or("SYSTEM_SUBTITLE_SOURCE_LANG", "en"),
            system_target_lang: env_or("SYSTEM_SUBTITLE_TARGET_LANG", "zh"),
            mic_source_lang: env_or("MIC_INTERPRETATION_SOURCE_LANG", "zh"),
            mic_target_lang: env_or("MIC_INTERPRETATION_TARGET_LANG", "en"),
        },
        system_segmenter: SystemSegmenterSettings {
            min_words_before_finalize: 10,
            preferred_max_chars: 140,
            hard_max_chars: 220,
            silence_for_visual_finalize_ms: 500,
            allow_comma_finalize: false,
            allow_short_pause_finalize: false,
        },
        mic_segmenter: MicSegmenterSettings {
            min_words_before_translate: 6,
            preferred_max_chars: 80,
            hard_max_chars: 140,
            silence_for_translate_ms: 800,
            allow_comma_translate: false,
        },
        diagnostics: DiagnosticsSettings {
            log_level: log_level.clone(),
            diagnostics_enabled,
            show_system_logs: true,
            show_mic_logs: true,
            show_asr_logs: true,
            show_translation_logs: true,
            show_tts_logs: true,
            show_error_logs: true,
        },
        log_level,
        diagnostics_enabled,
        e2e_autostart_system_subtitle: env_bool("APP_E2E_AUTOSTART_SYSTEM_SUBTITLE", false),
    }
}

pub fn load_app_settings() -> AppSettings {
    let preferred = settings_path().ok();
    let legacy = legacy_settings_path().ok();
    let mut settings = preferred
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
        .or_else(|| legacy.as_ref().and_then(|path| fs::read_to_string(path).ok()))
        .and_then(|text| serde_json::from_str::<AppSettings>(&text).ok())
        .unwrap_or_else(default_app_settings);
    refresh_secret_status(&mut settings);
    settings
}

pub fn save_app_settings_to_file(mut settings: AppSettings) -> Result<AppSettings, String> {
    validate_settings(&settings)?;
    refresh_secret_status(&mut settings);
    let text = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    fs::write(settings_path()?, text).map_err(|error| error.to_string())?;
    Ok(settings)
}

pub fn reset_app_settings_file() -> Result<AppSettings, String> {
    let path = settings_path()?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(default_app_settings())
}

pub fn get_secret_status() -> SecretStatus {
    load_dotenv_once();
    SecretStatus {
        game_tts_api_key_configured: env::var("GAME_TTS_API_KEY")
            .map(|value| !value.is_empty())
            .unwrap_or(false),
        dashscope_api_key_configured: env::var("DASHSCOPE_API_KEY")
            .map(|value| !value.is_empty())
            .unwrap_or(false),
    }
}

fn refresh_secret_status(settings: &mut AppSettings) {
    let secret_status = get_secret_status();
    settings.qwen_tts.game_tts_api_key_configured = secret_status.game_tts_api_key_configured;
    settings.qwen_tts.dashscope_api_key_configured = secret_status.dashscope_api_key_configured;
    settings.qwen_tts.has_api_key = !settings.qwen_tts.api_key.trim().is_empty()
        || secret_status.game_tts_api_key_configured
        || secret_status.dashscope_api_key_configured;
}

fn validate_settings(settings: &AppSettings) -> Result<(), String> {
    if settings.local_asr.ws_url.trim().is_empty() {
        return Err("本地 ASR 地址不能为空。".to_string());
    }
    if settings.local_asr.model_dir.trim().is_empty() {
        return Err("本地 ASR 模型目录未配置。".to_string());
    }
    if settings.local_asr.sample_rate == 0 {
        return Err("采样率必须大于 0。".to_string());
    }
    if settings.local_translation.timeout_ms == 0 {
        return Err("翻译超时时间必须大于 0。".to_string());
    }
    if settings.system_segmenter.hard_max_chars < settings.system_segmenter.preferred_max_chars {
        return Err("系统字幕硬上限不能小于推荐字符数。".to_string());
    }
    if settings.mic_segmenter.hard_max_chars < settings.mic_segmenter.preferred_max_chars {
        return Err("麦克风分段硬上限不能小于推荐字符数。".to_string());
    }
    Ok(())
}

pub fn tts_api_key() -> Option<String> {
    load_dotenv_once();
    let settings_key = load_app_settings().qwen_tts.api_key;
    if !settings_key.trim().is_empty() {
        return Some(settings_key);
    }
    env::var("GAME_TTS_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("DASHSCOPE_API_KEY")
                .ok()
                .filter(|value| !value.is_empty())
        })
}
