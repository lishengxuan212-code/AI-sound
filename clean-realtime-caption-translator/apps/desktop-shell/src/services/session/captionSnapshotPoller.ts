import { invoke } from '@tauri-apps/api/core';
import { systemSubtitleStore } from '../../stores/system-subtitle/systemSubtitleStore';
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
import { pushTip } from '../../stores/settings/settingsStore';
import { mergeSegmentText } from '../../utils/captionSegmenter';

interface RealtimeCaptionSnapshot {
  systemSessionId: string;
  systemUtteranceId: string;
  systemRawTranscript: string;
  systemTranslatedText: string;
  micSessionId: string;
  micUtteranceId: string;
  micRawTranscript: string;
  micTranslatedText: string;
  updatedAt: number;
}

let timer: number | undefined;
let lastUpdatedAt = 0;
let failureCount = 0;

export function startCaptionSnapshotPoller(): void {
  if (timer !== undefined) return;
  timer = window.setInterval(() => {
    void pollCaptionSnapshot();
  }, 200);
  void pollCaptionSnapshot();
}

async function pollCaptionSnapshot(): Promise<void> {
  try {
    const snapshot = await invoke<RealtimeCaptionSnapshot>('get_realtime_caption_snapshot');
    failureCount = 0;
    if (!snapshot.updatedAt || snapshot.updatedAt === lastUpdatedAt) return;
    lastUpdatedAt = snapshot.updatedAt;

    if (snapshot.systemRawTranscript) {
      systemSubtitleStore.currentRawCaption = mergeSegmentText(
        systemSubtitleStore.currentRawCaption,
        snapshot.systemRawTranscript,
      );
    }
    if (snapshot.systemTranslatedText) {
      systemSubtitleStore.currentTranslatedCaption = snapshot.systemTranslatedText;
      if (systemSubtitleStore.openVisualSegment) {
        systemSubtitleStore.openVisualSegment.translatedText = snapshot.systemTranslatedText;
      }
    }
    if (snapshot.micRawTranscript) {
      micInterpretationStore.currentRawTranscript = mergeSegmentText(
        micInterpretationStore.currentRawTranscript,
        snapshot.micRawTranscript,
      );
    }
    if (snapshot.micTranslatedText) {
      micInterpretationStore.currentTranslatedText = snapshot.micTranslatedText;
    }
  } catch (error) {
    failureCount += 1;
    if (failureCount === 1) {
      pushTip(`字幕快照轮询失败：${error instanceof Error ? error.message : String(error)}`);
    }
  }
}
