import { SessionKind } from './events';

export interface OpenVisualCaptionSegment {
  id: string;
  sessionKind: SessionKind;
  recognitionItemIds: string[];
  rawText: string;
  translatedText: string;
  startedAt: number;
  updatedAt: number;
  lastCompletedAt?: number;
  finalizeReason?: string;
}

export interface FinalizedCaptionSegment {
  id: string;
  sessionKind: SessionKind;
  recognitionItemIds: string[];
  rawText: string;
  translatedText: string;
  startedAt: number;
  endedAt: number;
  finalizeReason: string;
}
