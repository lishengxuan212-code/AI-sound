import { SessionKind } from './events';

export type TtsStatus = 'queued' | 'synthesizing' | 'audio_saved' | 'playing' | 'completed' | 'failed';

export interface TtsItem {
  id: string;
  sessionKind: SessionKind.MicInterpretation;
  translationItemId: string;
  text: string;
  model: 'qwen-qwen-tts-latest';
  voice: string;
  format: string;
  sampleRate?: number;
  status: TtsStatus;
  audioPath?: string;
  audioUrl?: string;
  fileSize?: number;
  error?: string;
  createdAt: number;
  completedAt?: number;
}
