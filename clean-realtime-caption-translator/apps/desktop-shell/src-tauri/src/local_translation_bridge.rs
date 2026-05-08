use crate::audio_session::SessionKind;
use crate::diagnostics::diagnostic;
use crate::settings::load_app_settings;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

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

    let client = reqwest::Client::new();
    let result = match client
        .post(endpoint)
        .timeout(Duration::from_secs(15))
        .json(&request)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => parse_translation_response(response, &request).await,
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

async fn parse_translation_response(response: reqwest::Response, request: &LocalTranslationRequest) -> LocalTranslationResponse {
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

async fn http_error_response(response: reqwest::Response, request: &LocalTranslationRequest) -> LocalTranslationResponse {
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
    use super::{build_translate_url, config_missing_response, LOCAL_TRANSLATION_ENDPOINT_MISSING};

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
        assert_eq!(result.error.as_deref(), Some(LOCAL_TRANSLATION_ENDPOINT_MISSING));
    }
}
