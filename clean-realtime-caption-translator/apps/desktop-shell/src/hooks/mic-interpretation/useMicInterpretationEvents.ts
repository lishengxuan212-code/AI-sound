import { listen } from '@tauri-apps/api/event';
import { assertSessionEvent, SessionKind, type SessionEvent } from '../../types/events';
import { useMicInterpretation } from './useMicInterpretation';
import type { TranslationItem } from '../../types/translation';
import type { TtsStatus } from '../../types/tts';

export async function bindMicInterpretationEvents(): Promise<() => void> {
  const mic = useMicInterpretation();
  const unlisten = await listen<SessionEvent>('session_event', (event) => {
    const payload = event.payload;
    assertSessionEvent(payload);
    if (payload.sessionKind !== SessionKind.MicInterpretation) return;
    const body = payload.payload as Record<string, unknown>;
    if (payload.type === 'mic_asr_partial') mic.handleMicAsrPartial(String(body.utteranceId), String(body.text));
    if (payload.type === 'mic_asr_final') mic.handleMicAsrFinal(String(body.utteranceId), String(body.text));
    if (payload.type === 'mic_translation_final') mic.handleMicTranslationFinal(body as unknown as TranslationItem);
    if (payload.type === 'mic_tts_status') {
      mic.handleMicTtsStatus(
        String(body.ttsId),
        String(body.status) as TtsStatus,
        body.error ? String(body.error) : undefined,
        body.translationItemId ? String(body.translationItemId) : undefined,
        body.audioPath ? String(body.audioPath) : undefined,
      );
    }
  });
  return unlisten;
}
