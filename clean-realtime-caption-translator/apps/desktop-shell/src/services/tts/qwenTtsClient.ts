import { invoke } from '@tauri-apps/api/core';
import { SessionKind } from '../../types/events';

export interface QwenTtsInvokeRequest {
  sessionKind: SessionKind.MicInterpretation;
  translationItemId: string;
  text: string;
  targetLang: string;
}

export function synthesizeQwenTts(request: QwenTtsInvokeRequest): Promise<void> {
  return invoke('synthesize_tts', { request });
}
