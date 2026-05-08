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
      );
    }
  });
  return unlisten;
}
