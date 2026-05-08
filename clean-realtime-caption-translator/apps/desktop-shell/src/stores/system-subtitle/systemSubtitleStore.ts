import { reactive } from 'vue';
import type { OpenVisualCaptionSegment, FinalizedCaptionSegment } from '../../types/captions';
import type { TranslationItem } from '../../types/translation';
import type { RecognitionItem } from '../../types/asr';

export interface SystemSubtitleStore {
  isRunning: boolean;
  audioLevel: number;
  currentRawCaption: string;
  currentTranslatedCaption: string;
  openVisualSegment?: OpenVisualCaptionSegment;
  recognitionItems: Record<string, RecognitionItem>;
  finalizedCaptions: FinalizedCaptionSegment[];
  translationQueue: TranslationItem[];
  errors: string[];
  diagnostics: string[];
}

export const systemSubtitleStore = reactive<SystemSubtitleStore>({
  isRunning: false,
  audioLevel: 0,
  currentRawCaption: '',
  currentTranslatedCaption: '',
  recognitionItems: {},
  finalizedCaptions: [],
  translationQueue: [],
  errors: [],
  diagnostics: [],
});
