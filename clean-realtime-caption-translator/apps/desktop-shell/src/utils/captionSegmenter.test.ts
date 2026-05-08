import { describe, expect, it } from 'vitest';
import { createCaptionSegmenter } from './captionSegmenter';
import { SessionKind } from '../types/events';

describe('captionSegmenter', () => {
  it('does not finalize a visual segment just because ASR final arrives', () => {
    const segmenter = createCaptionSegmenter('system');
    const result = segmenter.applyRecognition({
      id: 'r1',
      sessionKind: SessionKind.SystemSubtitle,
      provider: 'local-sherpa-onnx-realtime',
      rawInterimText: '',
      rawFinalText: 'hello world',
      isCompleted: true,
      startedAt: 1,
      updatedAt: 2,
    });

    expect(result.finalizedSegment).toBeUndefined();
    expect(result.openSegment.rawText).toBe('hello world');
  });

  it('uses independent microphone thresholds', () => {
    const system = createCaptionSegmenter('system');
    const mic = createCaptionSegmenter('mic');

    expect(system.config.hardMaxChars).toBeGreaterThan(mic.config.hardMaxChars);
    expect(system.config.silenceMs).toBeGreaterThan(mic.config.silenceMs);
  });
});
