export enum SessionKind {
  SystemSubtitle = 'SystemSubtitle',
  MicInterpretation = 'MicInterpretation',
}

export type SessionEventType =
  | 'system_asr_partial'
  | 'system_asr_final'
  | 'system_subtitle_partial'
  | 'system_subtitle_final'
  | 'system_translation_status'
  | 'system_translation_final'
  | 'system_segment_finalized'
  | 'mic_asr_partial'
  | 'mic_asr_final'
  | 'mic_translation_status'
  | 'mic_translation_final'
  | 'mic_tts_status'
  | 'mic_tts_playing'
  | 'mic_tts_completed'
  | 'mic_tts_error'
  | 'session_error'
  | 'audio_level';

export interface SessionEvent<T = unknown> {
  type: SessionEventType | string;
  sessionKind: SessionKind;
  sessionId?: string;
  payload?: T;
}

export function assertSessionEvent(event: unknown): event is SessionEvent {
  if (!event || typeof event !== 'object') {
    throw new Error('Session event must be an object with sessionKind');
  }
  const candidate = event as { sessionKind?: unknown };
  if (candidate.sessionKind !== SessionKind.SystemSubtitle && candidate.sessionKind !== SessionKind.MicInterpretation) {
    throw new Error('Session event missing valid sessionKind');
  }
  return true;
}
