use crate::audio_session::SessionKind;
use crate::diagnostics::diagnostic;
use crate::settings::load_app_settings;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

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
        return LocalTranslationResponse {
            request_id: request.request_id,
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some("本地翻译 endpoint 未配置".to_string()),
        };
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
        Ok(response) if response.status().is_success() => {
            response.json::<LocalTranslationResponse>().await.unwrap_or(LocalTranslationResponse {
                request_id: request.request_id.clone(),
                translated_text: String::new(),
                status: "failed".to_string(),
                error: Some("本地翻译服务返回格式无效".to_string()),
            })
        }
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let message = if status.as_u16() == 404 || status.as_u16() == 503 {
                "本地翻译模型未配置或未启动".to_string()
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
    use super::build_translate_url;

    #[test]
    fn builds_translate_url_from_base_endpoint() {
        assert_eq!(
            build_translate_url("http://127.0.0.1:9001").as_deref(),
            Some("http://127.0.0.1:9001/translate")
        );
        assert_eq!(
            build_translate_url("http://127.0.0.1:9001/").as_deref(),
            Some("http://127.0.0.1:9001/translate")
        );
    }

    #[test]
    fn preserves_explicit_translate_url() {
        assert_eq!(
            build_translate_url("http://127.0.0.1:9001/api/translate").as_deref(),
            Some("http://127.0.0.1:9001/api/translate")
        );
    }

    #[test]
    fn rejects_unconfigured_endpoint() {
        assert!(build_translate_url("").is_none());
        assert!(build_translate_url("   ").is_none());
    }
}
