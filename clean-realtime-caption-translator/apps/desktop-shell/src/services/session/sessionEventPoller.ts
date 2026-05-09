import { invoke } from '@tauri-apps/api/core';
import type { SessionEvent } from '../../types/events';
import { handleSystemSubtitleSessionEvent } from '../../hooks/system-subtitle/useSystemSubtitleEvents';
import { handleMicInterpretationSessionEvent } from '../../hooks/mic-interpretation/useMicInterpretationEvents';
import { pushTip } from '../../stores/settings/settingsStore';

interface StoredSessionEvent extends SessionEvent {
  sequence: number;
}

let lastSequence = 0;
let timer: number | undefined;
let failureCount = 0;

export function startSessionEventPoller(): void {
  if (timer !== undefined) return;
  timer = window.setInterval(() => {
    void pollSessionEvents();
  }, 250);
  void pollSessionEvents();
}

async function pollSessionEvents(): Promise<void> {
  try {
    const events = await invoke<StoredSessionEvent[]>('drain_session_events', { afterSequence: lastSequence });
    failureCount = 0;
    for (const event of events) {
      lastSequence = Math.max(lastSequence, event.sequence);
      handleSystemSubtitleSessionEvent(event);
      handleMicInterpretationSessionEvent(event);
    }
  } catch (error) {
    failureCount += 1;
    if (failureCount === 1) {
      pushTip(`字幕事件轮询失败：${error instanceof Error ? error.message : String(error)}`);
    }
  }
}
