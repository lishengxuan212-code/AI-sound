import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { assertSessionEvent, SessionKind, type SessionEvent } from '../../types/events';
import { useSystemSubtitle } from './useSystemSubtitle';
import type { TranslationItem } from '../../types/translation';
import { logDiagnostic } from '../../services/diagnostics/diagnosticsLogger';
import { pushTip } from '../../stores/settings/settingsStore';

const handledEvents = new Set<string>();

export function handleSystemSubtitleSessionEvent(payload: SessionEvent): void {
  const system = useSystemSubtitle();
  assertSessionEvent(payload);
  if (payload.sessionKind !== SessionKind.SystemSubtitle) return;
  const body = payload.payload as Record<string, unknown>;
  const key = [
    payload.type,
    payload.sessionId ?? '',
    body.utteranceId ?? body.id ?? '',
    body.text ?? body.status ?? '',
  ].join(':');
  if (handledEvents.has(key)) return;
  handledEvents.add(key);
  if (handledEvents.size > 500) handledEvents.clear();

  if (payload.type === 'system_asr_partial' || payload.type === 'system_subtitle_partial') {
    system.handleSystemAsrPartial(String(body.utteranceId), String(body.text));
  }
  if (payload.type === 'system_asr_final' || payload.type === 'system_subtitle_final') {
    system.handleSystemAsrFinal(String(body.utteranceId), String(body.text));
  }
  if (payload.type === 'system_translation_final') system.handleSystemTranslationFinal(body as unknown as TranslationItem);
  if (payload.type === 'session_error' && body.localAsrConnected === true) {
    logDiagnostic('[SYS][ASR_CONNECTED]', { sessionKind: SessionKind.SystemSubtitle, sessionId: payload.sessionId });
  }
  if (payload.type === 'session_error' && body.message) {
    system.store.errors.unshift(String(body.message));
    logDiagnostic('[ERROR][LOCAL_ASR_NOT_CONNECTED]', { message: body.message, sessionId: payload.sessionId });
  }
}

export async function bindSystemSubtitleEvents(): Promise<() => void> {
  const onEvent = (event: { payload: SessionEvent }): void => {
    try {
      handleSystemSubtitleSessionEvent(event.payload);
    } catch (error) {
      pushTip(`系统字幕事件处理失败：${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const appUnlisten = await listen<SessionEvent>('session_event', onEvent);
  const windowUnlisten = await getCurrentWebviewWindow().listen<SessionEvent>('session_event', onEvent);
  return () => {
    appUnlisten();
    windowUnlisten();
  };
}
