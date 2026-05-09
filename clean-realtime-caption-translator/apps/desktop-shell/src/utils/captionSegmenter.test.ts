import { describe, expect, it } from 'vitest';
import { createCaptionSegmenter, mergeSegmentText } from './captionSegmenter';
import { SessionKind } from '../types/events';

describe('captionSegmenter', () => {
  it('shows ASR draft frames in the visible visual segment without finalizing', () => {
    const segmenter = createCaptionSegmenter('system');
    const result = segmenter.applyRecognition({
      id: 'r-draft',
      sessionKind: SessionKind.SystemSubtitle,
      provider: 'local-sherpa-onnx-realtime',
      rawInterimText: 'what is happening',
      rawFinalText: '',
      isCompleted: false,
      startedAt: 1,
      updatedAt: 2,
    });

    expect(result.openSegment?.rawText).toBe('what is happening');
    expect(result.finalizedSegment).toBeUndefined();
    expect(segmenter.getOpenSegment()?.rawText).toBe('what is happening');
  });

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
    expect(result.openSegment?.rawText).toBe('hello world');
  });

  it('replaces draft text with the matching final text instead of duplicating it', () => {
    const segmenter = createCaptionSegmenter('system');

    segmenter.applyRecognition({
      id: 'r1',
      sessionKind: SessionKind.SystemSubtitle,
      provider: 'local-sherpa-onnx-realtime',
      rawInterimText: 'hello',
      rawFinalText: '',
      isCompleted: false,
      startedAt: 1,
      updatedAt: 2,
    });
    const result = segmenter.applyRecognition({
      id: 'r1',
      sessionKind: SessionKind.SystemSubtitle,
      provider: 'local-sherpa-onnx-realtime',
      rawInterimText: 'hello',
      rawFinalText: 'hello world',
      isCompleted: true,
      startedAt: 1,
      updatedAt: 3,
    });

    expect(result.openSegment?.rawText).toBe('hello world');
  });

  it('keeps the full current sentence when a snapshot only repeats the latest words', () => {
    expect(
      mergeSegmentText(
        'downstairs and go back down to the room that was long as i picked up so that i think that unlocked that that long',
        'that that long',
      ),
    ).toBe(
      'downstairs and go back down to the room that was long as i picked up so that i think that unlocked that that long',
    );
  });

  it('uses independent microphone thresholds', () => {
    const system = createCaptionSegmenter('system');
    const mic = createCaptionSegmenter('mic');

    expect(system.config.hardMaxChars).toBeGreaterThan(mic.config.hardMaxChars);
    expect(system.config.silenceMs).toBeGreaterThan(mic.config.silenceMs);
  });
});
