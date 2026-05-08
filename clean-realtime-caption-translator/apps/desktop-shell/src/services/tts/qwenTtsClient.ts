import { invoke } from '@tauri-apps/api/core';
import { SessionKind } from '../../types/events';

export interface QwenTtsInvokeRequest {
  sessionKind: SessionKind.MicInterpretation;
  ttsId: string;
  translationItemId: string;
  text: string;
  targetLang: string;
}

export function synthesizeQwenTts(request: QwenTtsInvokeRequest): Promise<void> {
  return invoke('synthesize_tts', { request });
}

export interface RetryTtsRequest {
  ttsId: string;
}

export function retryTtsPlayback(request: RetryTtsRequest): Promise<void> {
  return invoke('replay_tts_audio', { request });
}

export function stopTtsPlayback(ttsId?: string): Promise<void> {
  return invoke('stop_tts_audio', { request: { ttsId } });
}
