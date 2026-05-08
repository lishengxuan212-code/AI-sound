import { invoke } from '@tauri-apps/api/core';
import { SessionKind } from '../../types/events';

export function startAudioCapture(sessionKind: SessionKind): Promise<void> {
  return invoke('start_session', { sessionKind });
}

export function stopAudioCapture(sessionKind: SessionKind): Promise<void> {
  return invoke('stop_session', { sessionKind });
}
