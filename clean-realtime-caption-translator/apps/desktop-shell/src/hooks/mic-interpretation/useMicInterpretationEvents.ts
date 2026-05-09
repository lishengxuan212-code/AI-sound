import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { assertSessionEvent, SessionKind, type SessionEvent } from '../../types/events';
import { useMicInterpretation } from './useMicInterpretation';
import type { TranslationItem } from '../../types/translation';
import type { TtsStatus } from '../../types/tts';
import { logDiagnostic } from '../../services/diagnostics/diagnosticsLogger';
import { pushTip } from '../../stores/settings/settingsStore';

const handledEvents = new Set<string>();

export function handleMicInterpretationSessionEvent(payload: SessionEvent): void {
  const mic = useMicInterpretation();
  assertSessionEvent(payload);
  if (payload.sessionKind !== SessionKind.MicInterpretation) return;
  const body = payload.payload as Record<string, unknown>;
  const key = [
    payload.type,
    payload.sessionId ?? '',
    body.utteranceId ?? body.ttsId ?? body.id ?? '',
    body.text ?? body.status ?? '',
  ].join(':');
  if (handledEvents.has(key)) return;
  handledEvents.add(key);
  if (handledEvents.size > 500) handledEvents.clear();

  if (payload.type === 'mic_asr_partial') mic.handleMicAsrPartial(String(body.utteranceId), String(body.text));
  if (payload.type === 'mic_asr_final') mic.handleMicAsrFinal(String(body.utteranceId), String(body.text));
  if (payload.type === 'mic_translation_final') mic.handleMicTranslationFinal(body as unknown as TranslationItem);
  if (payload.type === 'mic_tts_status' || payload.type === 'mic_tts_playing' || payload.type === 'mic_tts_completed' || payload.type === 'mic_tts_error') {
    mic.handleMicTtsStatus(
      String(body.ttsId),
      String(body.status) as TtsStatus,
      body.error ? String(body.error) : undefined,
      body.translationItemId ? String(body.translationItemId) : undefined,
      body.audioPath ? String(body.audioPath) : undefined,
      body.audioUrl ? String(body.audioUrl) : undefined,
      typeof body.fileSize === 'number' ? body.fileSize : undefined,
      typeof body.sampleRate === 'number' ? body.sampleRate : undefined,
      body.format ? String(body.format) : undefined,
      body.model ? String(body.model) : undefined,
      body.voice ? String(body.voice) : undefined,
    );
  }
  if (payload.type === 'session_error' && body.localAsrConnected === true) {
    logDiagnostic('[MIC][ASR_CONNECTED]', { sessionKind: SessionKind.MicInterpretation, sessionId: payload.sessionId });
  }
  if (payload.type === 'session_error' && body.message) {
    mic.store.errors.unshift(String(body.message));
    logDiagnostic('[ERROR][LOCAL_ASR_NOT_CONNECTED]', { message: body.message, sessionId: payload.sessionId });
  }
}

export async function bindMicInterpretationEvents(): Promise<() => void> {
  const onEvent = (event: { payload: SessionEvent }): void => {
    try {
      handleMicInterpretationSessionEvent(event.payload);
    } catch (error) {
      pushTip(`麦克风同传事件处理失败：${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const appUnlisten = await listen<SessionEvent>('session_event', onEvent);
  const windowUnlisten = await getCurrentWebviewWindow().listen<SessionEvent>('session_event', onEvent);
  return () => {
    appUnlisten();
    windowUnlisten();
  };
}
