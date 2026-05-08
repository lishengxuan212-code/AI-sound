import { SessionKind } from './events';

export interface RecognitionItem {
  id: string;
  sessionKind: SessionKind;
  provider: 'local-sherpa-onnx-realtime' | 'local-vosk-realtime' | string;
  rawInterimText: string;
  rawFinalText: string;
  isCompleted: boolean;
  startedAt: number;
  updatedAt: number;
  sourceStartMs?: number;
  sourceEndMs?: number;
}

export interface LocalAsrEvent {
  sessionKind: SessionKind;
  utteranceId: string;
  text: string;
  isFinal: boolean;
  provider: string;
  sourceStartMs?: number;
  sourceEndMs?: number;
}
