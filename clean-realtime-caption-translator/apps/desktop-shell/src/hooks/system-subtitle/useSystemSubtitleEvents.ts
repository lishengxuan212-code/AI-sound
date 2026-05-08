import { listen } from '@tauri-apps/api/event';
import { assertSessionEvent, SessionKind, type SessionEvent } from '../../types/events';
import { useSystemSubtitle } from './useSystemSubtitle';
import type { TranslationItem } from '../../types/translation';

export async function bindSystemSubtitleEvents(): Promise<() => void> {
  const system = useSystemSubtitle();
  const unlisten = await listen<SessionEvent>('session_event', (event) => {
    const payload = event.payload;
    assertSessionEvent(payload);
    if (payload.sessionKind !== SessionKind.SystemSubtitle) return;
    const body = payload.payload as Record<string, unknown>;
    if (payload.type === 'system_subtitle_partial') system.handleSystemAsrPartial(String(body.utteranceId), String(body.text));
    if (payload.type === 'system_subtitle_final') system.handleSystemAsrFinal(String(body.utteranceId), String(body.text));
    if (payload.type === 'system_translation_final') system.handleSystemTranslationFinal(body as unknown as TranslationItem);
  });
  return unlisten;
}
