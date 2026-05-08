import { reactive } from 'vue';
import type { RecognitionItem } from '../../types/asr';
import type { TranslationItem } from '../../types/translation';
import type { TtsItem, TtsStatus } from '../../types/tts';

export interface MicInterpretationHistoryItem {
  recognitionItemIds: string[];
  rawText: string;
  translatedText: string;
  ttsStatus: TtsStatus;
  createdAt: number;
}

export interface MicInterpretationStore {
  isRunning: boolean;
  audioLevel: number;
  currentRawTranscript: string;
  currentTranslatedText: string;
  recognitionItems: Record<string, RecognitionItem>;
  translationQueue: TranslationItem[];
  ttsQueue: TtsItem[];
  ttsStatus: TtsStatus | 'idle';
  history: MicInterpretationHistoryItem[];
  errors: string[];
  diagnostics: string[];
}

export const micInterpretationStore = reactive<MicInterpretationStore>({
  isRunning: false,
  audioLevel: 0,
  currentRawTranscript: '',
  currentTranslatedText: '',
  recognitionItems: {},
  translationQueue: [],
  ttsQueue: [],
  ttsStatus: 'idle',
  history: [],
  errors: [],
  diagnostics: [],
});
