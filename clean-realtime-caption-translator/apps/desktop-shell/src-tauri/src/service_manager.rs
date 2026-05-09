use crate::local_translation_bridge::is_dashscope_provider;
use crate::settings::{load_app_settings, ConnectionTestResult};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::{sleep, timeout, Instant};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

const SERVICE_READY_TIMEOUT_MS: u64 = 120_000;
const SERVICE_POLL_INTERVAL_MS: u64 = 1_000;

#[derive(Debug, Clone, Serialize)]
pub struct LocalServicesStatus {
    #[serde(rename = "asrOk")]
    pub asr_ok: bool,
    #[serde(rename = "translationOk")]
    pub translation_ok: bool,
    #[serde(rename = "startedAsr")]
    pub started_asr: bool,
    #[serde(rename = "startedTranslation")]
    pub started_translation: bool,
    pub message: String,
}

impl LocalServicesStatus {
    pub fn ready(&self) -> bool {
        self.asr_ok && self.translation_ok
    }
}

pub async fn ensure_local_services_ready() -> Result<LocalServicesStatus, String> {
    let root = find_project_root()?;
    let mut status = probe_local_services().await;
    let mut started_asr = false;
    let mut started_translation = false;

    if !status.asr_ok {
        start_script(&root, "start_local_asr.ps1")?;
        started_asr = true;
    }
    if !status.translation_ok {
        start_script(&root, "start_local_translation.ps1")?;
        started_translation = true;
    }

    if !status.ready() {
        let deadline = Instant::now() + Duration::from_millis(SERVICE_READY_TIMEOUT_MS);
        loop {
            status = probe_local_services().await;
            if status.ready() || Instant::now() >= deadline {
                break;
            }
            sleep(Duration::from_millis(SERVICE_POLL_INTERVAL_MS)).await;
        }
    }

    status.started_asr = started_asr;
    status.started_translation = started_translation;
    status.message = if status.ready() {
        "本地 ASR 与翻译服务已就绪。".to_string()
    } else {
        format!(
            "本地服务尚未就绪：ASR={}，翻译={}",
            if status.asr_ok {
                "已连接"
            } else {
                "未连接"
            },
            if status.translation_ok {
                "已连接"
            } else {
                "未连接"
            }
        )
    };

    Ok(status)
}

pub async fn probe_local_services() -> LocalServicesStatus {
    let (asr, translation) = tokio::join!(probe_asr(), probe_translation());
    LocalServicesStatus {
        asr_ok: asr.ok,
        translation_ok: translation.ok,
        started_asr: false,
        started_translation: false,
        message: if asr.ok && translation.ok {
            "本地服务已就绪。".to_string()
        } else {
            format!("ASR：{}；翻译：{}", asr.message, translation.message)
        },
    }
}

async fn probe_asr() -> ConnectionTestResult {
    let settings = load_app_settings();
    if settings.local_asr.model_dir.trim().is_empty() {
        return ConnectionTestResult {
            ok: false,
            message: "ASR 模型目录未配置。".to_string(),
        };
    }
    match connect_async(&settings.local_asr.ws_url).await {
        Ok((mut socket, _response)) => {
            let health = json!({ "type": "health" }).to_string();
            if socket.send(Message::Text(health)).await.is_err() {
                return ConnectionTestResult {
                    ok: false,
                    message: "ASR 服务已连接，但健康检查发送失败。".to_string(),
                };
            }
            match timeout(Duration::from_secs(2), socket.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(value)
                            if value
                                .get("ok")
                                .and_then(|item| item.as_bool())
                                .unwrap_or(false)
                                && value
                                    .get("provider")
                                    .and_then(|item| item.as_str())
                                    .map(|provider| {
                                        provider.contains(settings.local_asr.provider.as_str())
                                    })
                                    .unwrap_or(false) =>
                        {
                            ConnectionTestResult {
                                ok: true,
                                message: "连接成功。".to_string(),
                            }
                        }
                        Ok(value)
                            if value
                                .get("ok")
                                .and_then(|item| item.as_bool())
                                .unwrap_or(false) =>
                        {
                            ConnectionTestResult {
                                ok: false,
                                message: format!(
                                    "ASR 服务引擎不匹配：当前={}，期望={}",
                                    value
                                        .get("provider")
                                        .and_then(|item| item.as_str())
                                        .unwrap_or("unknown"),
                                    settings.local_asr.provider
                                ),
                            }
                        }
                        Ok(value) => ConnectionTestResult {
                            ok: false,
                            message: value
                                .get("error")
                                .and_then(|item| item.as_str())
                                .unwrap_or("ASR 服务未就绪。")
                                .to_string(),
                        },
                        Err(_) => ConnectionTestResult {
                            ok: false,
                            message: "ASR 服务健康检查返回格式无效。".to_string(),
                        },
                    }
                }
                Ok(_) | Err(_) => ConnectionTestResult {
                    ok: false,
                    message: "ASR 服务健康检查无响应。".to_string(),
                },
            }
        }
        Err(_) => ConnectionTestResult {
            ok: false,
            message: "ASR 服务未启动。".to_string(),
        },
    }
}

async fn probe_translation() -> ConnectionTestResult {
    let settings = load_app_settings();
    if is_dashscope_provider(&settings.local_translation.provider) {
        if settings.local_translation.endpoint.trim().is_empty() {
            return ConnectionTestResult {
                ok: false,
                message: "百炼 API 地址未配置。".to_string(),
            };
        }
        return ConnectionTestResult {
            ok: true,
            message: "百炼 API 翻译已启用。".to_string(),
        };
    }

    if settings.local_translation.endpoint.trim().is_empty() {
        return ConnectionTestResult {
            ok: false,
            message: "翻译 endpoint 未配置。".to_string(),
        };
    }
    if settings.local_translation.model_path.trim().is_empty()
        && settings.local_translation.model_name.trim().is_empty()
    {
        return ConnectionTestResult {
            ok: false,
            message: "本地翻译模型未配置。".to_string(),
        };
    }

    let endpoint = settings
        .local_translation
        .endpoint
        .trim()
        .trim_end_matches('/');
    let health_url = format!("{endpoint}/health");
    match Client::new()
        .get(health_url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => ConnectionTestResult {
            ok: true,
            message: "连接成功。".to_string(),
        },
        Ok(_) | Err(_) => ConnectionTestResult {
            ok: false,
            message: "本地翻译服务未启动。".to_string(),
        },
    }
}

fn start_script(root: &Path, script_name: &str) -> Result<(), String> {
    let script = root.join("scripts").join(script_name);
    if !script.exists() {
        return Err(format!("找不到本地服务启动脚本：{}", script.display()));
    }
    let logs_dir = root.join("logs");
    fs::create_dir_all(&logs_dir)
        .map_err(|error| format!("创建本地服务日志目录失败：{}", error))?;
    let log_file = logs_dir.join(format!("{}.log", script_name.trim_end_matches(".ps1")));
    let stdout = File::create(&log_file).map_err(|error| {
        format!(
            "创建本地服务日志文件失败：{}: {}",
            log_file.display(),
            error
        )
    })?;
    let stderr = stdout
        .try_clone()
        .map_err(|error| format!("打开本地服务错误日志失败：{}", error))?;

    let mut command = Command::new(powershell_exe());
    command
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass");
    if cfg!(target_os = "windows") {
        command.arg("-WindowStyle").arg("Hidden");
    }
    command
        .arg("-File")
        .arg(&script)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("启动 {} 失败：{}", script.display(), error))
}

fn powershell_exe() -> &'static str {
    if cfg!(target_os = "windows") {
        "powershell.exe"
    } else {
        "pwsh"
    }
}

fn find_project_root() -> Result<PathBuf, String> {
    let current = std::env::current_dir().map_err(|error| error.to_string())?;
    find_project_root_from(&current)
        .ok_or_else(|| format!("无法定位项目根目录，当前目录为：{}", current.display()))
}

fn find_project_root_from(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|candidate| {
        let asr = candidate.join("scripts").join("start_local_asr.ps1");
        let translation = candidate
            .join("scripts")
            .join("start_local_translation.ps1");
        if asr.exists() && translation.exists() {
            Some(candidate.to_path_buf())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::find_project_root_from;
    use std::path::Path;

    #[test]
    fn project_root_search_accepts_current_workspace_layout() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|path| path.join("scripts").join("start_local_asr.ps1").exists())
            .expect("workspace root with service scripts");

        let nested = root.join("apps").join("desktop-shell").join("src-tauri");

        assert_eq!(find_project_root_from(&nested).as_deref(), Some(root));
    }
}
