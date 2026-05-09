import { beforeEach, describe, expect, it } from 'vitest';
import { useMicInterpretation } from './useMicInterpretation';
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
import { SessionKind } from '../../types/events';

function resetMicStore(): void {
  micInterpretationStore.currentRawTranscript = '';
  micInterpretationStore.currentTranslatedText = '';
  micInterpretationStore.recognitionItems = {};
  micInterpretationStore.translationQueue = [];
  micInterpretationStore.ttsQueue = [];
  micInterpretationStore.ttsStatus = 'idle';
  micInterpretationStore.history = [];
  micInterpretationStore.errors = [];
}

describe('useMicInterpretation', () => {
  beforeEach(() => {
    resetMicStore();
  });

  it('renders microphone ASR partial text immediately in the mic subtitle area', () => {
    const mic = useMicInterpretation();

    mic.handleMicAsrPartial('utt-1', 'hello from microphone');

    expect(micInterpretationStore.currentRawTranscript).toBe('hello from microphone');
  });

  it('replaces the current mic subtitle when new speech starts', () => {
    const mic = useMicInterpretation();

    mic.handleMicAsrPartial('utt-1', 'first sentence');
    mic.handleMicTranslationFinal({
      id: 'tr-1',
      sessionKind: SessionKind.MicInterpretation,
      recognitionItemIds: ['utt-1'],
      sourceText: 'first sentence',
      translatedText: '第一句话',
      sourceLang: 'en',
      targetLang: 'zh',
      status: 'completed',
      startedAt: 1,
    });
    mic.handleMicAsrPartial('utt-2', 'second sentence');

    expect(micInterpretationStore.currentRawTranscript).toBe('second sentence');
    expect(micInterpretationStore.currentTranslatedText).toBe('');
  });

  it('stores completed mic interpretation pairs in history', () => {
    const mic = useMicInterpretation();

    mic.handleMicTranslationFinal({
      id: 'tr-1',
      sessionKind: SessionKind.MicInterpretation,
      recognitionItemIds: ['utt-1'],
      sourceText: 'hello',
      translatedText: '你好',
      sourceLang: 'en',
      targetLang: 'zh',
      status: 'completed',
      startedAt: 1,
    });

    expect(micInterpretationStore.history).toHaveLength(1);
    expect(micInterpretationStore.history[0]).toMatchObject({
      rawText: 'hello',
      translatedText: '你好',
      ttsStatus: 'queued',
    });
  });
});
