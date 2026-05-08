import type { RecognitionItem } from '../../types/asr';
import type { TranslationItem } from '../../types/translation';
import type { TtsItem, TtsStatus } from '../../types/tts';
import { SessionKind } from '../../types/events';
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
import { startAudioCapture, stopAudioCapture } from '../../services/audio/audioCaptureService';
import { createCaptionSegmenter } from '../../utils/captionSegmenter';
import { createId } from '../../utils/id';
import { nowMs } from '../../utils/time';
import { logDiagnostic } from '../../services/diagnostics/diagnosticsLogger';

const segmenter = createCaptionSegmenter('mic');

export function useMicInterpretation() {
  async function startMicInterpretation(): Promise<void> {
    micInterpretationStore.isRunning = true;
    await startAudioCapture(SessionKind.MicInterpretation);
  }

  async function stopMicInterpretation(): Promise<void> {
    micInterpretationStore.isRunning = false;
    await stopAudioCapture(SessionKind.MicInterpretation);
  }

  function upsertRecognition(id: string, text: string, isFinal: boolean): RecognitionItem {
    const existing = micInterpretationStore.recognitionItems[id];
    const item: RecognitionItem =
      existing ??
      {
        id,
        sessionKind: SessionKind.MicInterpretation,
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
    micInterpretationStore.recognitionItems[id] = item;
    return item;
  }

  function handleMicAsrPartial(utteranceId: string, text: string): void {
    const item = upsertRecognition(utteranceId, text, false);
    const result = segmenter.applyRecognition(item);
    micInterpretationStore.currentRawTranscript = result.openSegment.rawText;
    logDiagnostic('[MIC][ASR_PARTIAL]', {
      sessionKind: SessionKind.MicInterpretation,
      utteranceId,
      textLength: text.length,
    });
  }

  function handleMicAsrFinal(utteranceId: string, text: string): TranslationItem {
    const item = upsertRecognition(utteranceId, text, true);
    const result = segmenter.applyRecognition(item);
    micInterpretationStore.currentRawTranscript = result.openSegment.rawText;
    const translation: TranslationItem = {
      id: createId('mic_translation'),
      sessionKind: SessionKind.MicInterpretation,
      recognitionItemIds: [item.id],
      sourceText: item.rawFinalText,
      translatedText: '',
      sourceLang: 'en',
      targetLang: 'zh',
      status: 'pending',
      startedAt: nowMs(),
    };
    micInterpretationStore.translationQueue.push(translation);
    logDiagnostic('[MIC][ASR_FINAL]', {
      sessionKind: SessionKind.MicInterpretation,
      utteranceId,
      translationId: translation.id,
      textLength: text.length,
    });
    return translation;
  }

  function handleMicTranslationFinal(item: TranslationItem): void {
    item.status = item.error ? 'failed' : 'completed';
    item.completedAt = nowMs();
    const existing = micInterpretationStore.translationQueue.find((queued) => queued.id === item.id);
    if (existing) Object.assign(existing, item);
    if (item.error) micInterpretationStore.errors.unshift(item.error);
    if (!item.translatedText) return;
    micInterpretationStore.currentTranslatedText = item.translatedText;
    const ttsItem: TtsItem = {
      id: createId('mic_tts'),
      sessionKind: SessionKind.MicInterpretation,
      translationItemId: item.id,
      text: item.translatedText,
      model: 'qwen-qwen-tts-latest',
      voice: '',
      format: 'wav',
      status: 'queued',
      createdAt: nowMs(),
    };
    micInterpretationStore.ttsQueue.push(ttsItem);
    micInterpretationStore.ttsStatus = 'queued';
    logDiagnostic('[MIC][TRANSLATION]', {
      sessionKind: SessionKind.MicInterpretation,
      translationId: item.id,
      ttsId: ttsItem.id,
      translationStatus: item.status,
    });
  }

  function handleMicTtsStatus(
    ttsId: string,
    status: TtsStatus,
    error?: string,
    translationItemId?: string,
    audioPath?: string,
    audioUrl?: string,
    fileSize?: number,
    sampleRate?: number,
    format?: string,
  ): void {
    let item = micInterpretationStore.ttsQueue.find((queued) => queued.id === ttsId);
    if (!item && translationItemId) {
      const translation = micInterpretationStore.translationQueue.find((queued) => queued.id === translationItemId);
      item = {
        id: ttsId,
        sessionKind: SessionKind.MicInterpretation,
        translationItemId,
        text: translation?.translatedText ?? '',
        model: 'qwen-qwen-tts-latest',
        voice: '',
        format: 'wav',
        status: 'queued',
        createdAt: nowMs(),
      };
      micInterpretationStore.ttsQueue.push(item);
    }
    if (item) {
      item.status = status;
      item.error = error;
      item.audioPath = audioPath ?? item.audioPath;
      item.audioUrl = audioUrl ?? item.audioUrl;
      item.fileSize = fileSize ?? item.fileSize;
      item.sampleRate = sampleRate ?? item.sampleRate;
      item.format = format ?? item.format;
      if (status === 'completed' || status === 'failed') item.completedAt = nowMs();
    }
    micInterpretationStore.ttsStatus = status;
    if (error) micInterpretationStore.errors.unshift(error);
    logDiagnostic('[MIC][TTS]', {
      sessionKind: SessionKind.MicInterpretation,
      ttsId,
      ttsModel: 'qwen-qwen-tts-latest',
      ttsStatus: status,
      error,
    });
  }

  return {
    store: micInterpretationStore,
    startMicInterpretation,
    stopMicInterpretation,
    handleMicAsrPartial,
    handleMicAsrFinal,
    handleMicTranslationFinal,
    handleMicTtsStatus,
  };
}
