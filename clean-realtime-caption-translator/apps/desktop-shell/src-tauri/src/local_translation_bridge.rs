use crate::audio_session::SessionKind;
use crate::diagnostics::diagnostic;
use crate::settings::load_app_settings;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, error::Error, time::Duration};

pub const LOCAL_TRANSLATION_ENDPOINT_MISSING: &str = "本地翻译 endpoint 未配置";
pub const LOCAL_TRANSLATION_MODEL_UNAVAILABLE: &str = "本地翻译模型未配置或未安装";

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalTranslationRequest {
    #[serde(rename = "requestId")]
    pub request_id: String,
    #[serde(rename = "sessionKind")]
    pub session_kind: SessionKind,
    #[serde(rename = "sourceText")]
    pub source_text: String,
    #[serde(rename = "sourceLang")]
    pub source_lang: String,
    #[serde(rename = "targetLang")]
    pub target_lang: String,
    #[serde(rename = "contextBefore")]
    pub context_before: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalTranslationResponse {
    #[serde(rename = "requestId")]
    pub request_id: String,
    #[serde(rename = "translatedText")]
    pub translated_text: String,
    pub status: String,
    pub error: Option<String>,
}

pub async fn translate_local(request: LocalTranslationRequest) -> LocalTranslationResponse {
    let settings = load_app_settings();
    if is_dashscope_provider(&settings.local_translation.provider) {
        return translate_dashscope(&settings.local_translation, request).await;
    }

    let Some(endpoint) = build_translate_url(&settings.local_translation.endpoint) else {
        diagnostic(
            "[LOCAL_TRANSLATION][CONFIG_MISSING]",
            json!({
                "sessionKind": request.session_kind,
                "translationId": request.request_id,
                "textLength": request.source_text.chars().count(),
                "translationStatus": "failed",
                "error": LOCAL_TRANSLATION_ENDPOINT_MISSING
            }),
        );
        return config_missing_response(&request.request_id);
    };

    diagnostic(
        translation_prefix(request.session_kind, "PENDING"),
        json!({
            "sessionKind": request.session_kind,
            "translationId": request.request_id,
            "textLength": request.source_text.chars().count(),
            "translationStatus": "pending"
        }),
    );

    let client = match reqwest::Client::builder().no_proxy().build() {
        Ok(client) => client,
        Err(error) => {
            return LocalTranslationResponse {
                request_id: request.request_id.clone(),
                translated_text: String::new(),
                status: "failed".to_string(),
                error: Some(format!("本地翻译 HTTP client 初始化失败: {error}")),
            };
        }
    };

    let result = match client
        .post(endpoint)
        .timeout(Duration::from_secs(15))
        .json(&request)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            parse_translation_response(response, &request).await
        }
        Ok(response) => http_error_response(response, &request).await,
        Err(error) => LocalTranslationResponse {
            request_id: request.request_id.clone(),
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some(format!("本地翻译服务连接失败: {error}")),
        },
    };

    diagnostic(
        translation_prefix(request.session_kind, "FINAL"),
        json!({
            "sessionKind": request.session_kind,
            "translationId": result.request_id,
            "textLength": request.source_text.chars().count(),
            "translationStatus": result.status,
            "error": result.error
        }),
    );
    result
}

async fn translate_dashscope(
    settings: &crate::settings::LocalTranslationSettings,
    request: LocalTranslationRequest,
) -> LocalTranslationResponse {
    let Some(endpoint) = build_dashscope_chat_url(&settings.endpoint) else {
        return LocalTranslationResponse {
            request_id: request.request_id,
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some("百炼 API 地址未配置。".to_string()),
        };
    };
    if settings.api_key.trim().is_empty() {
        return LocalTranslationResponse {
            request_id: request.request_id,
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some("百炼 API Key 未填写。".to_string()),
        };
    }
    if settings.model_name.trim().is_empty() {
        return LocalTranslationResponse {
            request_id: request.request_id,
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some("百炼模型名称未填写。".to_string()),
        };
    }

    diagnostic(
        translation_prefix(request.session_kind, "PENDING"),
        json!({
            "sessionKind": request.session_kind,
            "translationId": request.request_id,
            "textLength": request.source_text.chars().count(),
            "translationStatus": "pending",
            "provider": settings.provider,
            "endpoint": settings.endpoint,
            "proxy": proxy_diagnostic(&settings.proxy_url)
        }),
    );

    let context = if request.context_before.is_empty() {
        String::new()
    } else {
        format!(
            "Context before this sentence:\n{}\n\n",
            request.context_before.join("\n")
        )
    };
    let user_content = format!(
        "{}\n\n{}Hard requirement: translate from {} to {}. The target language is {}, even if the custom prompt mentions another language. Return only the translated text.\nText:\n{}",
        build_system_prompt(&settings.prompt),
        context,
        request.source_lang,
        request.target_lang,
        request.target_lang,
        request.source_text
    );

    let payload = json!({
        "model": settings.model_name,
        "messages": [
            {
                "role": "user",
                "content": user_content
            }
        ],
        "temperature": 0.2
    });

    let client = match build_cloud_client(&settings.proxy_url) {
        Ok(client) => client,
        Err(error) => {
            return LocalTranslationResponse {
                request_id: request.request_id,
                translated_text: String::new(),
                status: "failed".to_string(),
                error: Some(format!("百炼 HTTP client 初始化失败: {error}")),
            };
        }
    };

    let request_id_for_error = request.request_id.clone();
    let result = match client
        .post(endpoint)
        .bearer_auth(settings.api_key.trim())
        .timeout(Duration::from_millis(settings.timeout_ms.max(1)))
        .json(&payload)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            parse_dashscope_response(response, &request.request_id).await
        }
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            LocalTranslationResponse {
                request_id: request.request_id,
                translated_text: String::new(),
                status: "failed".to_string(),
                error: Some(format!("百炼翻译失败: HTTP {status}: {}", body.trim())),
            }
        }
        Err(error) => LocalTranslationResponse {
            request_id: request_id_for_error,
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some(format!(
                "百炼翻译连接失败: {}; endpoint={}; proxy={}",
                error_chain(&error),
                settings.endpoint.trim(),
                proxy_diagnostic(&settings.proxy_url)
            )),
        },
    };

    diagnostic(
        translation_prefix(request.session_kind, "FINAL"),
        json!({
            "sessionKind": request.session_kind,
            "translationId": result.request_id,
            "textLength": request.source_text.chars().count(),
            "translationStatus": result.status,
            "error": result.error
        }),
    );
    result
}

pub fn build_system_prompt(custom_prompt: &str) -> String {
    const BASE_PROMPT: &str = "You are a professional real-time subtitle translator. Preserve meaning, natural wording, and punctuation. Do not add explanations. The runtime source and target languages in the user message have highest priority.";
    let custom = custom_prompt.trim();
    if custom.is_empty() {
        BASE_PROMPT.to_string()
    } else {
        format!(
            "{BASE_PROMPT}\n\nUser style preference, unless it conflicts with the runtime language direction:\n{custom}"
        )
    }
}

fn build_cloud_client(proxy_url: &str) -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder()
        .http1_only()
        .pool_max_idle_per_host(0)
        .tcp_nodelay(true)
        .connect_timeout(Duration::from_secs(8));
    if let Some(proxy) = resolved_proxy_url(proxy_url) {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    builder.build()
}

fn resolved_proxy_url(proxy_url: &str) -> Option<String> {
    let configured = proxy_url.trim();
    if !configured.is_empty() {
        return Some(configured.to_string());
    }
    ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY"]
        .into_iter()
        .find_map(|key| env::var(key).ok().filter(|value| !value.trim().is_empty()))
}

fn proxy_diagnostic(proxy_url: &str) -> String {
    let configured = proxy_url.trim();
    if !configured.is_empty() {
        return format!("configured({configured})");
    }
    let env_proxy = ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY"]
        .into_iter()
        .find_map(|key| env::var(key).ok().filter(|value| !value.trim().is_empty()).map(|_| key));
    match env_proxy {
        Some(key) => format!("env({key})"),
        None => "none".to_string(),
    }
}

fn error_chain(error: &reqwest::Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(item) = source {
        parts.push(item.to_string());
        source = item.source();
    }
    parts.join(" | caused by: ")
}

async fn parse_dashscope_response(
    response: reqwest::Response,
    request_id: &str,
) -> LocalTranslationResponse {
    let parsed = response.json::<serde_json::Value>().await;
    match parsed {
        Ok(value) => {
            let translated = value
                .pointer("/choices/0/message/content")
                .and_then(|item| item.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if translated.is_empty() {
                return LocalTranslationResponse {
                    request_id: request_id.to_string(),
                    translated_text: String::new(),
                    status: "failed".to_string(),
                    error: Some("百炼翻译返回内容为空。".to_string()),
                };
            }
            LocalTranslationResponse {
                request_id: request_id.to_string(),
                translated_text: translated,
                status: "completed".to_string(),
                error: None,
            }
        }
        Err(error) => LocalTranslationResponse {
            request_id: request_id.to_string(),
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some(format!("百炼翻译返回格式无效: {error}")),
        },
    }
}

async fn parse_translation_response(
    response: reqwest::Response,
    request: &LocalTranslationRequest,
) -> LocalTranslationResponse {
    let parsed = response.json::<LocalTranslationResponse>().await;
    match parsed {
        Ok(mut result) if result.request_id == request.request_id => {
            if result.status != "completed" && result.status != "failed" {
                result.status = "failed".to_string();
                result.translated_text.clear();
                result.error = Some("本地翻译服务返回 status 无效".to_string());
            }
            result
        }
        Ok(result) => LocalTranslationResponse {
            request_id: request.request_id.clone(),
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some(format!(
                "本地翻译服务 requestId 不匹配: expected {}, got {}",
                request.request_id, result.request_id
            )),
        },
        Err(error) => LocalTranslationResponse {
            request_id: request.request_id.clone(),
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some(format!("本地翻译服务返回格式无效: {error}")),
        },
    }
}

async fn http_error_response(
    response: reqwest::Response,
    request: &LocalTranslationRequest,
) -> LocalTranslationResponse {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let message = if status.as_u16() == 404 || status.as_u16() == 503 {
        LOCAL_TRANSLATION_MODEL_UNAVAILABLE.to_string()
    } else {
        format!("本地翻译服务连接失败: HTTP {status}")
    };
    LocalTranslationResponse {
        request_id: request.request_id.clone(),
        translated_text: String::new(),
        status: "failed".to_string(),
        error: Some(if body.trim().is_empty() {
            message
        } else {
            format!("{message}: {}", body.trim())
        }),
    }
}

pub fn build_translate_url(endpoint: &str) -> Option<String> {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.ends_with("/translate") {
        Some(trimmed.to_string())
    } else {
        Some(format!("{trimmed}/translate"))
    }
}

pub fn build_dashscope_chat_url(endpoint: &str) -> Option<String> {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.ends_with("/chat/completions") {
        Some(trimmed.to_string())
    } else {
        Some(format!("{trimmed}/chat/completions"))
    }
}

pub fn is_dashscope_provider(provider: &str) -> bool {
    matches!(
        provider.trim().to_ascii_lowercase().as_str(),
        "dashscope" | "dashscope_openai" | "aliyun_bailian" | "bailian"
    )
}

pub fn config_missing_response(request_id: &str) -> LocalTranslationResponse {
    LocalTranslationResponse {
        request_id: request_id.to_string(),
        translated_text: String::new(),
        status: "failed".to_string(),
        error: Some(LOCAL_TRANSLATION_ENDPOINT_MISSING.to_string()),
    }
}

fn translation_prefix(session_kind: SessionKind, phase: &str) -> &'static str {
    match (session_kind, phase) {
        (SessionKind::SystemSubtitle, "PENDING") => "[SYS][TRANSLATION_PENDING]",
        (SessionKind::SystemSubtitle, _) => "[SYS][TRANSLATION_FINAL]",
        (SessionKind::MicInterpretation, "PENDING") => "[MIC][TRANSLATION_PENDING]",
        (SessionKind::MicInterpretation, _) => "[MIC][TRANSLATION_FINAL]",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_dashscope_chat_url, build_system_prompt, build_translate_url, config_missing_response,
        is_dashscope_provider, LOCAL_TRANSLATION_ENDPOINT_MISSING,
    };

    #[test]
    fn builds_translate_url_from_base_endpoint() {
        assert_eq!(
            build_translate_url("http://127.0.0.1:8777").as_deref(),
            Some("http://127.0.0.1:8777/translate")
        );
        assert_eq!(
            build_translate_url("http://127.0.0.1:8777/").as_deref(),
            Some("http://127.0.0.1:8777/translate")
        );
    }

    #[test]
    fn preserves_explicit_translate_url() {
        assert_eq!(
            build_translate_url("http://127.0.0.1:8777/api/translate").as_deref(),
            Some("http://127.0.0.1:8777/api/translate")
        );
    }

    #[test]
    fn builds_dashscope_chat_completions_url() {
        assert_eq!(
            build_dashscope_chat_url("https://dashscope.aliyuncs.com/compatible-mode/v1")
                .as_deref(),
            Some("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
        );
        assert_eq!(
            build_dashscope_chat_url(
                "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
            )
            .as_deref(),
            Some("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
        );
    }

    #[test]
    fn detects_dashscope_translation_provider_aliases() {
        assert!(is_dashscope_provider("dashscope_openai"));
        assert!(is_dashscope_provider("bailian"));
        assert!(!is_dashscope_provider("transformers_seq2seq"));
    }

    #[test]
    fn appends_custom_cloud_translation_prompt_to_system_prompt() {
        let prompt = build_system_prompt("Use concise gaming terminology.");

        assert!(prompt.contains("professional real-time subtitle translator"));
        assert!(prompt.contains("Use concise gaming terminology."));
    }

    #[test]
    fn rejects_unconfigured_endpoint() {
        assert!(build_translate_url("").is_none());
        assert!(build_translate_url("   ").is_none());
    }

    #[test]
    fn config_missing_response_preserves_request_id() {
        let result = config_missing_response("req-1");
        assert_eq!(result.request_id, "req-1");
        assert_eq!(result.status, "failed");
        assert_eq!(result.translated_text, "");
        assert_eq!(
            result.error.as_deref(),
            Some(LOCAL_TRANSLATION_ENDPOINT_MISSING)
        );
    }
}
