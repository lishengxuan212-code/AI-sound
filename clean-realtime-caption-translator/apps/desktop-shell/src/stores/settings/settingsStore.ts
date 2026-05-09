import { reactive } from 'vue';
import type { AppSettings, AudioDeviceSummary, TtsVoiceOption } from '../../types/settings';

export const settingsStore = reactive<{
  settings?: AppSettings;
  error?: string;
  saving: boolean;
  lastMessage?: string;
  tips: string[];
  inputDevices: AudioDeviceSummary[];
  outputDevices: AudioDeviceSummary[];
  ttsVoices: TtsVoiceOption[];
}>({
  saving: false,
  tips: [],
  inputDevices: [],
  outputDevices: [],
  ttsVoices: [],
});

export function pushTip(message: string): void {
  const text = message.trim();
  if (!text) return;
  settingsStore.tips.unshift(text);
  if (settingsStore.tips.length > 5) settingsStore.tips.splice(5);
}

export function dismissTip(index: number): void {
  settingsStore.tips.splice(index, 1);
}
