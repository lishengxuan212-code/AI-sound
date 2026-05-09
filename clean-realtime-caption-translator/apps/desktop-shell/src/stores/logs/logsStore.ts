import { computed, reactive } from 'vue';
import { redactSecrets } from '../../services/diagnostics/secretRedactor';
import type { SessionKind } from '../../types/events';

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';
export type LogModule = 'system' | 'mic' | 'asr' | 'translation' | 'tts' | 'settings' | 'audio' | 'diagnostics';

export interface LogEntry {
  id: string;
  timestamp: number;
  level: LogLevel;
  module: LogModule;
  sessionKind: SessionKind | null;
  sessionId?: string;
  eventType: string;
  message: string;
  details?: Record<string, unknown>;
  redacted: boolean;
}

export const moduleLabels: Record<LogModule, string> = {
  system: '系统字幕',
  mic: '麦克风同声传译',
  asr: '本地语音识别',
  translation: '本地翻译',
  tts: '语音合成',
  settings: '设置',
  audio: '音频采集',
  diagnostics: '诊断',
};

export const levelLabels: Record<LogLevel, string> = {
  debug: '调试',
  info: '信息',
  warn: '警告',
  error: '错误',
};

export const logsStore = reactive({
  entries: [] as LogEntry[],
  selectedId: '',
  moduleFilter: 'all' as LogModule | 'all' | 'error',
  levelFilter: 'all' as LogLevel | 'all',
  search: '',
});

export const filteredLogs = computed(() => {
  const keyword = logsStore.search.trim().toLowerCase();
  return logsStore.entries.filter((entry) => {
    const moduleMatches =
      logsStore.moduleFilter === 'all' ||
      (logsStore.moduleFilter === 'error' ? entry.level === 'error' : entry.module === logsStore.moduleFilter);
    const levelMatches = logsStore.levelFilter === 'all' || entry.level === logsStore.levelFilter;
    const details = JSON.stringify(entry.details ?? {}).toLowerCase();
    const searchMatches =
      !keyword ||
      entry.message.toLowerCase().includes(keyword) ||
      entry.eventType.toLowerCase().includes(keyword) ||
      (entry.sessionId ?? '').toLowerCase().includes(keyword) ||
      details.includes(keyword);
    return moduleMatches && levelMatches && searchMatches;
  });
});

export function addLogEntry(entry: Omit<LogEntry, 'id' | 'timestamp' | 'redacted'> & { id?: string; timestamp?: number }): LogEntry {
  const details = redactSecrets(entry.details ?? {}) as Record<string, unknown>;
  const safeEntry: LogEntry = {
    ...entry,
    id: entry.id ?? `log_${Date.now()}_${Math.random().toString(16).slice(2)}`,
    timestamp: entry.timestamp ?? Date.now(),
    details,
    redacted: true,
  };
  logsStore.entries.unshift(safeEntry);
  if (logsStore.entries.length > 1000) logsStore.entries.splice(1000);
  return safeEntry;
}

export function clearLogs(): void {
  logsStore.entries.splice(0);
  logsStore.selectedId = '';
}

export function exportLogs(format: 'json' | 'txt'): void {
  const safeEntries = redactSecrets(logsStore.entries);
  const content =
    format === 'json'
      ? JSON.stringify(safeEntries, null, 2)
      : logsStore.entries
          .map((entry) => {
            const time = new Date(entry.timestamp).toLocaleString('zh-CN');
            return `[${time}] [${levelLabels[entry.level]}] [${moduleLabels[entry.module]}] ${entry.eventType} ${entry.message} ${JSON.stringify(entry.details ?? {})}`;
          })
          .join('\n');
  const blob = new Blob([content], { type: format === 'json' ? 'application/json' : 'text/plain' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `diagnostics-logs.${format}`;
  link.click();
  URL.revokeObjectURL(url);
}
