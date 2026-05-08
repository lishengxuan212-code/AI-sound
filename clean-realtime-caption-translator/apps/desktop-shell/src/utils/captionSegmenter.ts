import type { FinalizedCaptionSegment, OpenVisualCaptionSegment } from '../types/captions';
import type { RecognitionItem } from '../types/asr';
import { SessionKind } from '../types/events';
import { createId } from './id';
import { compactText, countWords, hasStrongSentenceEnd } from './text';
import { nowMs } from './time';

export interface SegmenterConfig {
  minWords: number;
  preferredMaxChars: number;
  hardMaxChars: number;
  silenceMs: number;
  allowComma: boolean;
}

export type SegmenterKind = 'system' | 'mic';

const systemConfig: SegmenterConfig = {
  minWords: 10,
  preferredMaxChars: 140,
  hardMaxChars: 220,
  silenceMs: 1500,
  allowComma: false,
};

const micConfig: SegmenterConfig = {
  minWords: 6,
  preferredMaxChars: 80,
  hardMaxChars: 140,
  silenceMs: 900,
  allowComma: false,
};

export interface SegmenterResult {
  openSegment: OpenVisualCaptionSegment;
  finalizedSegment?: FinalizedCaptionSegment;
}

export interface CaptionSegmenter {
  readonly config: SegmenterConfig;
  applyRecognition(item: RecognitionItem): SegmenterResult;
  forceFinalize(reason: string): FinalizedCaptionSegment | undefined;
  getOpenSegment(): OpenVisualCaptionSegment | undefined;
}

export function createCaptionSegmenter(kind: SegmenterKind): CaptionSegmenter {
  const config = kind === 'system' ? systemConfig : micConfig;
  const sessionKind = kind === 'system' ? SessionKind.SystemSubtitle : SessionKind.MicInterpretation;
  let openSegment: OpenVisualCaptionSegment | undefined;

  function ensureOpen(startedAt: number): OpenVisualCaptionSegment {
    if (!openSegment) {
      openSegment = {
        id: createId(kind === 'system' ? 'sys_seg' : 'mic_seg'),
        sessionKind,
        recognitionItemIds: [],
        rawText: '',
        translatedText: '',
        startedAt,
        updatedAt: startedAt,
      };
    }
    return openSegment;
  }

  function shouldFinalize(text: string, item: RecognitionItem): string | undefined {
    if (!item.isCompleted) return undefined;
    const normalized = compactText(text);
    if (normalized.length >= config.hardMaxChars) return 'hard_max_length';
    if (normalized.length >= config.preferredMaxChars && hasStrongSentenceEnd(normalized)) return 'preferred_length_sentence_end';
    if (countWords(normalized) >= config.minWords && hasStrongSentenceEnd(normalized)) return 'sentence_end';
    return undefined;
  }

  function finalize(reason: string): FinalizedCaptionSegment | undefined {
    if (!openSegment || !openSegment.rawText) return undefined;
    const finalized: FinalizedCaptionSegment = {
      id: createId(kind === 'system' ? 'sys_final' : 'mic_final'),
      sessionKind,
      recognitionItemIds: [...openSegment.recognitionItemIds],
      rawText: openSegment.rawText,
      translatedText: openSegment.translatedText,
      startedAt: openSegment.startedAt,
      endedAt: nowMs(),
      finalizeReason: reason,
    };
    openSegment = undefined;
    return finalized;
  }

  return {
    config,
    applyRecognition(item: RecognitionItem): SegmenterResult {
      const segment = ensureOpen(item.startedAt);
      if (!segment.recognitionItemIds.includes(item.id)) {
        segment.recognitionItemIds.push(item.id);
      }
      const text = item.rawFinalText || item.rawInterimText;
      const previousItemsText = segment.rawText && !segment.rawText.includes(text) ? `${segment.rawText} ${text}` : text;
      segment.rawText = compactText(previousItemsText);
      segment.updatedAt = item.updatedAt;
      if (item.isCompleted) segment.lastCompletedAt = item.updatedAt;

      const reason = shouldFinalize(segment.rawText, item);
      return {
        openSegment: segment,
        finalizedSegment: reason ? finalize(reason) : undefined,
      };
    },
    forceFinalize: finalize,
    getOpenSegment() {
      return openSegment;
    },
  };
}
