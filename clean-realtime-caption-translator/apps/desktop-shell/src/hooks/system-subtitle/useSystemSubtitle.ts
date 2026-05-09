import type { RecognitionItem } from '../../types/asr';
import type { TranslationItem } from '../../types/translation';
import type { FinalizedCaptionSegment } from '../../types/captions';
import { SessionKind } from '../../types/events';
import { systemSubtitleStore } from '../../stores/system-subtitle/systemSubtitleStore';
import { startAudioCapture, stopAudioCapture } from '../../services/audio/audioCaptureService';
import { areLocalServicesReady, localServicesStore } from '../../stores/services/localServicesStore';
import { createCaptionSegmenter } from '../../utils/captionSegmenter';
import { nowMs } from '../../utils/time';
import { logDiagnostic } from '../../services/diagnostics/diagnosticsLogger';

const segmenter = createCaptionSegmenter('system');

export function useSystemSubtitle() {
  async function startSystemSubtitle(): Promise<void> {
    if (!areLocalServicesReady()) {
      systemSubtitleStore.errors.unshift(localServicesStore.message || '本地服务尚未就绪。');
      return;
    }
    logDiagnostic('[SYS][START]', {});
    try {
      await startAudioCapture(SessionKind.SystemSubtitle);
      systemSubtitleStore.isRunning = true;
    } catch (error) {
      systemSubtitleStore.isRunning = false;
      systemSubtitleStore.errors.unshift(error instanceof Error ? error.message : String(error));
    }
  }

  async function stopSystemSubtitle(): Promise<void> {
    systemSubtitleStore.isRunning = false;
    const finalized = segmenter.forceFinalize('manual_stop');
    if (finalized) systemSubtitleStore.finalizedCaptions.unshift(finalized);
    logDiagnostic('[SYS][STOP]', {});
    await stopAudioCapture(SessionKind.SystemSubtitle);
  }

  function upsertRecognition(id: string, text: string, isFinal: boolean): RecognitionItem {
    const existing = systemSubtitleStore.recognitionItems[id];
    const item: RecognitionItem =
      existing ??
      {
        id,
        sessionKind: SessionKind.SystemSubtitle,
        provider: 'local-vosk-realtime',
        rawInterimText: '',
        rawFinalText: '',
        isCompleted: false,
        startedAt: nowMs(),
        updatedAt: nowMs(),
      };

    item.rawInterimText = isFinal ? item.rawInterimText : text;
    item.rawFinalText = isFinal ? text : item.rawFinalText;
    item.isCompleted = isFinal;
    item.updatedAt = nowMs();
    systemSubtitleStore.recognitionItems[id] = item;
    return item;
  }

  function handleSystemAsrPartial(utteranceId: string, text: string): void {
    const item = upsertRecognition(utteranceId, text, false);
    const result = segmenter.applyRecognition(item);
    if (result.openSegment) {
      systemSubtitleStore.openVisualSegment = result.openSegment;
      systemSubtitleStore.currentRawCaption = result.openSegment.rawText;
    }
    logDiagnostic('[SYS][ASR_PARTIAL]', {
      sessionKind: SessionKind.SystemSubtitle,
      utteranceId,
      textLength: text.length,
      openSegmentId: result.openSegment?.id,
      renderPolicy: 'realtime_draft',
    });
  }

  function handleSystemAsrFinal(utteranceId: string, text: string): void {
    const item = upsertRecognition(utteranceId, text, true);
    const result = segmenter.applyRecognition(item);
    if (result.openSegment) {
      systemSubtitleStore.openVisualSegment = result.openSegment;
    }
    if (result.finalizedSegment) systemSubtitleStore.finalizedCaptions.unshift(result.finalizedSegment);
    logDiagnostic('[SYS][ASR_FINAL]', {
      sessionKind: SessionKind.SystemSubtitle,
      utteranceId,
      textLength: text.length,
      openSegmentId: result.openSegment?.id,
      finalizeReason: result.finalizedSegment?.finalizeReason,
      renderPolicy: 'await_translation_final',
    });
  }

  function handleSystemTranslationFinal(item: TranslationItem): void {
    item.status = item.error ? 'failed' : 'completed';
    item.completedAt = nowMs();
    const existing = systemSubtitleStore.translationQueue.find((queued) => queued.id === item.id);
    if (existing) {
      if (existing.status !== 'completed') Object.assign(existing, item);
    } else {
      systemSubtitleStore.translationQueue.push(item);
    }
    if (item.error) systemSubtitleStore.errors.unshift(item.error);
    systemSubtitleStore.currentTranslatedCaption = item.translatedText || systemSubtitleStore.currentTranslatedCaption;
    if (systemSubtitleStore.openVisualSegment && item.translatedText) {
      systemSubtitleStore.openVisualSegment.rawText = item.sourceText || systemSubtitleStore.openVisualSegment.rawText;
      systemSubtitleStore.openVisualSegment.translatedText = item.translatedText;
    }
    if (!item.error && item.translatedText && !systemSubtitleStore.stableTranslatedCaptions.some((segment) => segment.id === item.id)) {
      const stableSegment: FinalizedCaptionSegment = {
        id: item.id,
        sessionKind: item.sessionKind,
        recognitionItemIds: [...item.recognitionItemIds],
        rawText: item.sourceText,
        translatedText: item.translatedText,
        startedAt: item.startedAt,
        endedAt: item.completedAt,
        finalizeReason: 'translated',
      };
      systemSubtitleStore.stableTranslatedCaptions.push(stableSegment);
    }
    logDiagnostic(item.error ? '[SYS][TRANSLATION_ERROR]' : '[SYS][TRANSLATION_FINAL]', {
      sessionKind: SessionKind.SystemSubtitle,
      translationId: item.id,
      translationStatus: item.status,
      textLength: item.sourceText.length,
      error: item.error,
    });
  }

  function finalizeSystemVisualSegment(): void {
    const finalized = segmenter.forceFinalize('manual');
    if (finalized) {
      systemSubtitleStore.finalizedCaptions.unshift(finalized);
      logDiagnostic('[SYS][SEGMENT_FINALIZED]', { segmentId: finalized.id, textLength: finalized.rawText.length });
    }
  }

  return {
    store: systemSubtitleStore,
    startSystemSubtitle,
    stopSystemSubtitle,
    handleSystemAsrPartial,
    handleSystemAsrFinal,
    handleSystemTranslationFinal,
    finalizeSystemVisualSegment,
  };
}
