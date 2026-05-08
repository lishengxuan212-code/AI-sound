import type { RecognitionItem } from '../../types/asr';
import type { TranslationItem } from '../../types/translation';
import { SessionKind } from '../../types/events';
import { systemSubtitleStore } from '../../stores/system-subtitle/systemSubtitleStore';
import { startAudioCapture, stopAudioCapture } from '../../services/audio/audioCaptureService';
import { createCaptionSegmenter } from '../../utils/captionSegmenter';
import { createId } from '../../utils/id';
import { nowMs } from '../../utils/time';
import { logDiagnostic } from '../../services/diagnostics/diagnosticsLogger';

const segmenter = createCaptionSegmenter('system');

export function useSystemSubtitle() {
  async function startSystemSubtitle(): Promise<void> {
    systemSubtitleStore.isRunning = true;
    await startAudioCapture(SessionKind.SystemSubtitle);
  }

  async function stopSystemSubtitle(): Promise<void> {
    systemSubtitleStore.isRunning = false;
    const finalized = segmenter.forceFinalize('manual_stop');
    if (finalized) systemSubtitleStore.finalizedCaptions.unshift(finalized);
    await stopAudioCapture(SessionKind.SystemSubtitle);
  }

  function upsertRecognition(id: string, text: string, isFinal: boolean): RecognitionItem {
    const existing = systemSubtitleStore.recognitionItems[id];
    const item: RecognitionItem =
      existing ??
      {
        id,
        sessionKind: SessionKind.SystemSubtitle,
        provider: 'local-sherpa-onnx-realtime',
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
    systemSubtitleStore.openVisualSegment = result.openSegment;
    systemSubtitleStore.currentRawCaption = result.openSegment.rawText;
    logDiagnostic('[SYS][ASR_PARTIAL]', {
      sessionKind: SessionKind.SystemSubtitle,
      utteranceId,
      textLength: text.length,
      openSegmentId: result.openSegment.id,
    });
  }

  function handleSystemAsrFinal(utteranceId: string, text: string): TranslationItem {
    const item = upsertRecognition(utteranceId, text, true);
    const result = segmenter.applyRecognition(item);
    systemSubtitleStore.openVisualSegment = result.openSegment;
    systemSubtitleStore.currentRawCaption = result.openSegment.rawText;
    if (result.finalizedSegment) systemSubtitleStore.finalizedCaptions.unshift(result.finalizedSegment);

    const translation: TranslationItem = {
      id: createId('sys_translation'),
      sessionKind: SessionKind.SystemSubtitle,
      recognitionItemIds: [item.id],
      sourceText: item.rawFinalText,
      translatedText: '',
      sourceLang: 'en',
      targetLang: 'zh',
      status: 'pending',
      startedAt: nowMs(),
    };
    systemSubtitleStore.translationQueue.push(translation);
    logDiagnostic('[SYS][ASR_FINAL]', {
      sessionKind: SessionKind.SystemSubtitle,
      utteranceId,
      textLength: text.length,
      openSegmentId: result.openSegment.id,
      finalizeReason: result.finalizedSegment?.finalizeReason,
    });
    return translation;
  }

  function handleSystemTranslationFinal(item: TranslationItem): void {
    item.status = item.error ? 'failed' : 'completed';
    item.completedAt = nowMs();
    const existing = systemSubtitleStore.translationQueue.find((queued) => queued.id === item.id);
    if (existing) Object.assign(existing, item);
    if (item.error) systemSubtitleStore.errors.unshift(item.error);
    systemSubtitleStore.currentTranslatedCaption = item.translatedText || systemSubtitleStore.currentTranslatedCaption;
    if (systemSubtitleStore.openVisualSegment && item.translatedText) {
      systemSubtitleStore.openVisualSegment.translatedText = item.translatedText;
    }
    logDiagnostic('[SYS][TRANSLATION]', {
      sessionKind: SessionKind.SystemSubtitle,
      translationId: item.id,
      translationStatus: item.status,
      textLength: item.sourceText.length,
    });
  }

  function finalizeSystemVisualSegment(): void {
    const finalized = segmenter.forceFinalize('manual');
    if (finalized) systemSubtitleStore.finalizedCaptions.unshift(finalized);
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
