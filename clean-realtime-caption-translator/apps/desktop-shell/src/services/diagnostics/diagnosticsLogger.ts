import { redactSecrets } from './secretRedactor';
import { addLogEntry, type LogLevel, type LogModule } from '../../stores/logs/logsStore';
import { pushTip, settingsStore } from '../../stores/settings/settingsStore';
import { SessionKind } from '../../types/events';

export type LogPrefix =
  | '[SYS][START]'
  | '[SYS][STOP]'
  | '[SYS][AUDIO]'
  | '[SYS][ASR_CONNECTED]'
  | '[SYS][ASR_PARTIAL]'
  | '[SYS][ASR_FINAL]'
  | '[SYS][TRANSLATION_PENDING]'
  | '[SYS][TRANSLATION_FINAL]'
  | '[SYS][TRANSLATION_ERROR]'
  | '[SYS][TRANSLATION]'
  | '[SYS][SEGMENT_FINALIZED]'
  | '[SYS][ERROR]'
  | '[MIC][START]'
  | '[MIC][STOP]'
  | '[MIC][AUDIO]'
  | '[MIC][ASR_CONNECTED]'
  | '[MIC][ASR_PARTIAL]'
  | '[MIC][ASR_FINAL]'
  | '[MIC][TRANSLATION_PENDING]'
  | '[MIC][TRANSLATION_FINAL]'
  | '[MIC][TRANSLATION_ERROR]'
  | '[MIC][TRANSLATION]'
  | '[MIC][TTS]'
  | '[MIC][TTS_QUEUED]'
  | '[MIC][TTS_SYNTHESIZING]'
  | '[MIC][TTS_AUDIO_SAVED]'
  | '[MIC][TTS_PLAYING]'
  | '[MIC][TTS_COMPLETED]'
  | '[MIC][TTS_ERROR]'
  | '[SETTINGS][LOAD]'
  | '[SETTINGS][SAVE]'
  | '[SETTINGS][RESET]'
  | '[SETTINGS][TEST_ASR]'
  | '[SETTINGS][TEST_TRANSLATION]'
  | '[SETTINGS][TEST_TTS]'
  | '[SETTINGS][SECRET_STATUS]'
  | '[ERROR][LOCAL_ASR_NOT_CONNECTED]'
  | '[ERROR][LOCAL_TRANSLATION_ENDPOINT_MISSING]'
  | '[ERROR][LOCAL_TRANSLATION_SERVER_UNREACHABLE]'
  | '[ERROR][TTS_KEY_MISSING]'
  | '[ERROR][TTS_CONFIG_INVALID]'
  | '[ERROR][TTS_MODEL_NOT_FOUND]'
  | '[ERROR][TTS_AUDIO_SAVE_FAILED]'
  | '[ERROR][TTS_PLAYBACK_FAILED]'
  | '[CONFIG]'
  | '[SECRET_REDACTED]';

const prefixMeta: Partial<Record<LogPrefix, { module: LogModule; level: LogLevel; message: string; sessionKind: SessionKind | null }>> = {
  '[SYS][START]': { module: 'system', level: 'info', message: '系统字幕翻译已启动。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][STOP]': { module: 'system', level: 'info', message: '系统字幕翻译已停止。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][AUDIO]': { module: 'audio', level: 'debug', message: '系统音频采集事件。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][ASR_CONNECTED]': { module: 'asr', level: 'info', message: '系统字幕本地 ASR 已连接。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][ASR_PARTIAL]': { module: 'asr', level: 'debug', message: '系统字幕收到识别中间结果。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][ASR_FINAL]': { module: 'asr', level: 'info', message: '系统字幕收到识别最终结果。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][TRANSLATION_PENDING]': { module: 'translation', level: 'info', message: '系统字幕翻译中。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][TRANSLATION_FINAL]': { module: 'translation', level: 'info', message: '系统字幕翻译已完成。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][TRANSLATION_ERROR]': { module: 'translation', level: 'error', message: '系统字幕翻译失败，原文已保留。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][TRANSLATION]': { module: 'translation', level: 'info', message: '系统字幕翻译事件。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][SEGMENT_FINALIZED]': { module: 'system', level: 'info', message: '系统字幕分段已完成。', sessionKind: SessionKind.SystemSubtitle },
  '[SYS][ERROR]': { module: 'system', level: 'error', message: '系统字幕发生错误。', sessionKind: SessionKind.SystemSubtitle },
  '[MIC][START]': { module: 'mic', level: 'info', message: '麦克风同声传译已启动。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][STOP]': { module: 'mic', level: 'info', message: '麦克风同声传译已停止。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][AUDIO]': { module: 'audio', level: 'debug', message: '麦克风音频采集事件。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][ASR_CONNECTED]': { module: 'asr', level: 'info', message: '麦克风本地 ASR 已连接。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][ASR_PARTIAL]': { module: 'asr', level: 'debug', message: '麦克风收到识别中间结果。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][ASR_FINAL]': { module: 'asr', level: 'info', message: '麦克风收到识别最终结果。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TRANSLATION_PENDING]': { module: 'translation', level: 'info', message: '麦克风译文生成中。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TRANSLATION_FINAL]': { module: 'translation', level: 'info', message: '麦克风译文已完成。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TRANSLATION_ERROR]': { module: 'translation', level: 'error', message: '麦克风翻译失败，原文已保留。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TRANSLATION]': { module: 'translation', level: 'info', message: '麦克风翻译事件。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TTS]': { module: 'tts', level: 'info', message: 'TTS 事件。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TTS_QUEUED]': { module: 'tts', level: 'info', message: 'TTS 已加入队列。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TTS_SYNTHESIZING]': { module: 'tts', level: 'info', message: '语音合成中。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TTS_AUDIO_SAVED]': { module: 'tts', level: 'info', message: 'TTS 音频已保存。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TTS_PLAYING]': { module: 'tts', level: 'info', message: 'TTS 音频播放中。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TTS_COMPLETED]': { module: 'tts', level: 'info', message: 'TTS 播放已完成。', sessionKind: SessionKind.MicInterpretation },
  '[MIC][TTS_ERROR]': { module: 'tts', level: 'error', message: 'TTS 处理失败。', sessionKind: SessionKind.MicInterpretation },
  '[SETTINGS][LOAD]': { module: 'settings', level: 'info', message: '设置已加载。', sessionKind: null },
  '[SETTINGS][SAVE]': { module: 'settings', level: 'info', message: '设置已保存。', sessionKind: null },
  '[SETTINGS][RESET]': { module: 'settings', level: 'warn', message: '设置已恢复默认值。', sessionKind: null },
  '[SETTINGS][TEST_ASR]': { module: 'settings', level: 'info', message: '已测试本地 ASR 连接。', sessionKind: null },
  '[SETTINGS][TEST_TRANSLATION]': { module: 'settings', level: 'info', message: '已测试本地翻译连接。', sessionKind: null },
  '[SETTINGS][TEST_TTS]': { module: 'settings', level: 'info', message: '已测试 TTS 配置。', sessionKind: null },
  '[SETTINGS][SECRET_STATUS]': { module: 'settings', level: 'info', message: '已读取密钥配置状态。', sessionKind: null },
  '[ERROR][LOCAL_ASR_NOT_CONNECTED]': { module: 'asr', level: 'error', message: '本地 ASR 服务未启动，请先运行本地 ASR 服务。', sessionKind: null },
  '[ERROR][LOCAL_TRANSLATION_ENDPOINT_MISSING]': { module: 'translation', level: 'error', message: '本地翻译 endpoint 未配置。', sessionKind: null },
  '[ERROR][LOCAL_TRANSLATION_SERVER_UNREACHABLE]': { module: 'translation', level: 'error', message: '本地翻译服务未启动或连接失败。', sessionKind: null },
  '[ERROR][TTS_KEY_MISSING]': { module: 'tts', level: 'error', message: 'TTS API Key 未配置。', sessionKind: null },
  '[ERROR][TTS_CONFIG_INVALID]': { module: 'tts', level: 'error', message: 'TTS 配置不完整，请检查 API 地址、API Key 和模型名称。', sessionKind: null },
  '[ERROR][TTS_MODEL_NOT_FOUND]': { module: 'tts', level: 'error', message: 'TTS 模型不可用，请检查模型名称。', sessionKind: null },
  '[ERROR][TTS_AUDIO_SAVE_FAILED]': { module: 'tts', level: 'error', message: 'TTS 音频保存失败。', sessionKind: SessionKind.MicInterpretation },
  '[ERROR][TTS_PLAYBACK_FAILED]': { module: 'tts', level: 'error', message: 'TTS 音频播放失败。', sessionKind: SessionKind.MicInterpretation },
  '[CONFIG]': { module: 'settings', level: 'info', message: '应用配置已读取。', sessionKind: null },
  '[SECRET_REDACTED]': { module: 'diagnostics', level: 'debug', message: '日志内容已脱敏。', sessionKind: null },
};

function moduleEnabled(module: LogModule): boolean {
  const diagnostics = settingsStore.settings?.diagnostics;
  if (!diagnostics?.diagnosticsEnabled) return false;
  if (module === 'system') return diagnostics.showSystemLogs;
  if (module === 'mic') return diagnostics.showMicLogs;
  if (module === 'asr') return diagnostics.showAsrLogs;
  if (module === 'translation') return diagnostics.showTranslationLogs;
  if (module === 'tts') return diagnostics.showTtsLogs;
  if (module === 'audio') return diagnostics.showAsrLogs;
  return true;
}

function levelEnabled(level: LogLevel): boolean {
  const order: Record<LogLevel, number> = { debug: 0, info: 1, warn: 2, error: 3 };
  const configured = settingsStore.settings?.diagnostics.logLevel ?? 'info';
  return order[level] >= order[configured];
}

export function logDiagnostic(prefix: LogPrefix, details: object = {}): void {
  const safe = redactSecrets(details as Record<string, unknown>);
  const meta = prefixMeta[prefix] ?? { module: 'diagnostics', level: 'info' as LogLevel, message: '诊断事件。', sessionKind: null };

  if (meta.level === 'error') {
    const detailMessage = typeof safe.message === 'string' && safe.message ? ` ${safe.message}` : '';
    pushTip(`${meta.message}${detailMessage}`);
  }

  if (!moduleEnabled(meta.module) || !levelEnabled(meta.level)) return;
  if (meta.level === 'error' && !settingsStore.settings?.diagnostics.showErrorLogs) return;

  addLogEntry({
    level: meta.level,
    module: meta.module,
    sessionKind: meta.sessionKind,
    sessionId: typeof safe.sessionId === 'string' ? safe.sessionId : undefined,
    eventType: prefix,
    message: meta.message,
    details: safe,
  });
}
