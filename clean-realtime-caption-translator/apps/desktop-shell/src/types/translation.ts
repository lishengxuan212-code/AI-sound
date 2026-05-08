import { SessionKind } from './events';

export type TranslationStatus = 'pending' | 'translating' | 'completed' | 'failed';

export interface TranslationItem {
  id: string;
  sessionKind: SessionKind;
  recognitionItemIds: string[];
  sourceText: string;
  translatedText: string;
  sourceLang: string;
  targetLang: string;
  status: TranslationStatus;
  error?: string;
  startedAt: number;
  completedAt?: number;
}

export interface LocalTranslationRequest {
  requestId: string;
  sessionKind: SessionKind;
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  contextBefore: string[];
}

export interface LocalTranslationResponse {
  requestId: string;
  translatedText: string;
  status: TranslationStatus;
  error?: string;
}
