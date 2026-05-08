import { SessionKind } from './events';

export type TtsStatus = 'queued' | 'synthesizing' | 'playing' | 'completed' | 'failed';

export interface TtsItem {
  id: string;
  sessionKind: SessionKind.MicInterpretation;
  translationItemId: string;
  text: string;
  model: 'qwen-qwen-tts-latest';
  voice: string;
  format: string;
  status: TtsStatus;
  audioPath?: string;
  audioUrl?: string;
  error?: string;
  createdAt: number;
  completedAt?: number;
}
