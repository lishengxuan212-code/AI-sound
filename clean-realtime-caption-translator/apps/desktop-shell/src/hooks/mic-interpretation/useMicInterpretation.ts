import type { RecognitionItem } from '../../types/asr';
import type { TranslationItem } from '../../types/translation';
import type { TtsItem, TtsStatus } from '../../types/tts';
import { SessionKind } from '../../types/events';
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
import { settingsStore } from '../../stores/settings/settingsStore';
import { areLocalServicesReady, localServicesStore } from '../../stores/services/localServicesStore';
import { startAudioCapture, stopAudioCapture } from '../../services/audio/audioCaptureService';
import { createCaptionSegmenter } from '../../utils/captionSegmenter';
import { createId } from '../../utils/id';
import { nowMs } from '../../utils/time';
import { logDiagnostic } from '../../services/diagnostics/diagnosticsLogger';

const segmenter = createCaptionSegmenter('mic');

function renderMicRawTranscript(text: string): void {
  const next = text.trim();
  if (!next) return;
  micInterpretationStore.currentRawTranscript = next;
}

export function useMicInterpretation() {
  async function startMicInterpretation(): Promise<void> {
    if (!areLocalServicesReady()) {
      micInterpretationStore.errors.unshift(localServicesStore.message || '本地服务尚未就绪。');
      return;
    }
    logDiagnostic('[MIC][START]', {});
    try {
      await startAudioCapture(SessionKind.MicInterpretation);
      micInterpretationStore.isRunning = true;
    } catch (error) {
      micInterpretationStore.isRunning = false;
      micInterpretationStore.errors.unshift(error instanceof Error ? error.message : String(error));
    }
  }

  async function stopMicInterpretation(): Promise<void> {
    micInterpretationStore.isRunning = false;
    logDiagnostic('[MIC][STOP]', {});
    await stopAudioCapture(SessionKind.MicInterpretation);
  }

  function upsertRecognition(id: string, text: string, isFinal: boolean): RecognitionItem {
    const existing = micInterpretationStore.recognitionItems[id];
    const item: RecognitionItem =
      existing ??
      {
        id,
        sessionKind: SessionKind.MicInterpretation,
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
    micInterpretationStore.recognitionItems[id] = item;
    return item;
  }

  function handleMicAsrPartial(utteranceId: string, text: string): void {
    upsertRecognition(utteranceId, text, false);
    renderMicRawTranscript(text);
    micInterpretationStore.currentTranslatedText = '';
    logDiagnostic('[MIC][ASR_PARTIAL]', {
      sessionKind: SessionKind.MicInterpretation,
      utteranceId,
      textLength: text.length,
      renderPolicy: 'realtime_draft',
    });
  }

  function handleMicAsrFinal(utteranceId: string, text: string): void {
    const item = upsertRecognition(utteranceId, text, true);
    const result = segmenter.applyRecognition(item);
    renderMicRawTranscript(result.openSegment?.rawText || text);
    logDiagnostic('[MIC][ASR_FINAL]', {
      sessionKind: SessionKind.MicInterpretation,
      utteranceId,
      textLength: text.length,
      openSegmentId: result.openSegment?.id,
      finalizeReason: result.finalizedSegment?.finalizeReason,
      renderPolicy: 'await_translation_final',
    });
  }

  function handleMicTranslationFinal(item: TranslationItem): void {
    item.status = item.error ? 'failed' : 'completed';
    item.completedAt = nowMs();
    const existing = micInterpretationStore.translationQueue.find((queued) => queued.id === item.id);
    if (existing) Object.assign(existing, item);
    else micInterpretationStore.translationQueue.push(item);
    if (item.error) micInterpretationStore.errors.unshift(item.error);
    logDiagnostic(item.error ? '[MIC][TRANSLATION_ERROR]' : '[MIC][TRANSLATION_FINAL]', {
      sessionKind: SessionKind.MicInterpretation,
      translationId: item.id,
      translationStatus: item.status,
      error: item.error,
    });
    if (!item.translatedText) return;
    micInterpretationStore.currentRawTranscript = item.sourceText || micInterpretationStore.currentRawTranscript;
    micInterpretationStore.currentTranslatedText = item.translatedText;
    micInterpretationStore.history.unshift({
      recognitionItemIds: item.recognitionItemIds,
      rawText: item.sourceText,
      translatedText: item.translatedText,
      ttsStatus: 'queued',
      createdAt: item.completedAt ?? nowMs(),
    });
    if (micInterpretationStore.history.length > 50) micInterpretationStore.history.splice(50);
    const ttsItem: TtsItem = {
      id: createId('mic_tts'),
      sessionKind: SessionKind.MicInterpretation,
      translationItemId: item.id,
      text: item.translatedText,
      model: settingsStore.settings?.qwenTts.model ?? '',
      voice: settingsStore.settings?.qwenTts.voice ?? '',
      format: settingsStore.settings?.qwenTts.format ?? 'wav',
      status: 'queued',
      createdAt: nowMs(),
    };
    micInterpretationStore.ttsQueue.push(ttsItem);
    micInterpretationStore.ttsStatus = 'queued';
    logDiagnostic('[MIC][TTS_QUEUED]', {
      sessionKind: SessionKind.MicInterpretation,
      translationId: item.id,
      ttsId: ttsItem.id,
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
    model?: string,
    voice?: string,
  ): void {
    let item = micInterpretationStore.ttsQueue.find((queued) => queued.id === ttsId);
    if (!item && translationItemId) {
      const translation = micInterpretationStore.translationQueue.find((queued) => queued.id === translationItemId);
      item = {
        id: ttsId,
        sessionKind: SessionKind.MicInterpretation,
        translationItemId,
        text: translation?.translatedText ?? '',
        model: model ?? settingsStore.settings?.qwenTts.model ?? '',
        voice: voice ?? settingsStore.settings?.qwenTts.voice ?? '',
        format: format ?? settingsStore.settings?.qwenTts.format ?? 'wav',
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
      item.model = model ?? item.model;
      item.voice = voice ?? item.voice;
      if (status === 'completed' || status === 'failed') item.completedAt = nowMs();
    }
    micInterpretationStore.ttsStatus = status;
    if (error) micInterpretationStore.errors.unshift(error);
    const prefix =
      status === 'synthesizing'
        ? '[MIC][TTS_SYNTHESIZING]'
        : status === 'audio_saved'
          ? '[MIC][TTS_AUDIO_SAVED]'
          : status === 'playing'
            ? '[MIC][TTS_PLAYING]'
            : status === 'completed'
              ? '[MIC][TTS_COMPLETED]'
              : status === 'failed'
                ? '[MIC][TTS_ERROR]'
                : '[MIC][TTS_QUEUED]';
    logDiagnostic(prefix, {
      sessionKind: SessionKind.MicInterpretation,
      ttsId,
      ttsModel: item?.model ?? settingsStore.settings?.qwenTts.model ?? '',
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
