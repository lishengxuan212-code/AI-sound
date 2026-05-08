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

export interface RetryTtsRequest {
  ttsId: string;
  translationItemId: string;
  audioPath: string;
}

export function retryTtsPlayback(request: RetryTtsRequest): Promise<void> {
  return invoke('retry_tts', { request });
}
