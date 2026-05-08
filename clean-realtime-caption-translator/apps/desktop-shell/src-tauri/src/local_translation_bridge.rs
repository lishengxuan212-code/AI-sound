use crate::audio_session::SessionKind;
use crate::settings::load_app_settings;
use serde::{Deserialize, Serialize};
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
    pub context_before: Option<String>,
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
    if settings.local_translation.endpoint.is_empty() {
        return LocalTranslationResponse {
            request_id: request.request_id,
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some("本地翻译模型未配置".to_string()),
        };
    }

    let client = reqwest::Client::new();
    match client
        .post(&settings.local_translation.endpoint)
        .timeout(Duration::from_secs(15))
        .json(&request)
        .send()
        .await
    {
        Ok(response) => response.json::<LocalTranslationResponse>().await.unwrap_or(LocalTranslationResponse {
            request_id: request.request_id,
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some("本地翻译服务返回格式无效".to_string()),
        }),
        Err(error) => LocalTranslationResponse {
            request_id: request.request_id,
            translated_text: String::new(),
            status: "failed".to_string(),
            error: Some(error.to_string()),
        },
    }
}
