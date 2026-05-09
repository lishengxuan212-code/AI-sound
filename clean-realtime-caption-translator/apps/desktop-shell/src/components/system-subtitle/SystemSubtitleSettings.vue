<script setup lang="ts">
import { settingsStore } from '../../stores/settings/settingsStore';
import { saveAppSettings } from '../../services/settings/settingsManager';
import { useSystemSubtitle } from '../../hooks/system-subtitle/useSystemSubtitle';

const EN_ASR_MODEL_DIR = 'local-asr\\models\\vosk-model-small-en-us-0.15';
const ZH_ASR_MODEL_DIR = 'local-asr\\models\\sherpa-onnx-streaming-paraformer-bilingual-zh-en';

const system = useSystemSubtitle();
const languageDirections = [
  { label: '中文 -> 英文', source: 'zh', target: 'en', asrModelDir: ZH_ASR_MODEL_DIR },
  { label: '英文 -> 中文', source: 'en', target: 'zh', asrModelDir: EN_ASR_MODEL_DIR },
];

let persistTimer: number | undefined;
let restarting = false;

async function persistSettings(): Promise<void> {
  if (!settingsStore.settings) return;
  settingsStore.settings = await saveAppSettings(settingsStore.settings);
}

function persistSettingsSoon(): void {
  if (persistTimer) window.clearTimeout(persistTimer);
  persistTimer = window.setTimeout(() => {
    persistTimer = undefined;
    void persistSettings();
  }, 250);
}

async function changeDirection(event: Event): Promise<void> {
  if (!settingsStore.settings || restarting) return;
  const value = String((event.target as HTMLSelectElement).value);
  const direction = languageDirections.find((item) => `${item.source}->${item.target}` === value);
  if (!direction) return;

  const wasRunning = system.store.isRunning;
  restarting = true;
  try {
    settingsStore.settings.language.systemSourceLang = direction.source;
    settingsStore.settings.language.systemTargetLang = direction.target;
    settingsStore.settings.localAsr.provider = 'vosk';
    settingsStore.settings.localAsr.modelDir = direction.asrModelDir;
    await persistSettings();

    if (wasRunning) {
      await system.stopSystemSubtitle();
      await system.startSystemSubtitle();
    }
  } finally {
    restarting = false;
  }
}
</script>

<template>
  <section v-if="settingsStore.settings" class="inline-settings">
    <label>翻译方向
      <select
        :value="`${settingsStore.settings.language.systemSourceLang}->${settingsStore.settings.language.systemTargetLang}`"
        :disabled="restarting"
        @change="changeDirection"
      >
        <option v-for="item in languageDirections" :key="`${item.source}-${item.target}`" :value="`${item.source}->${item.target}`">
          {{ item.label }}
        </option>
      </select>
    </label>
    <label>自动分段翻译间隔(ms)
      <input
        v-model.number="settingsStore.settings.systemSegmenter.silenceForVisualFinalizeMs"
        type="number"
        min="200"
        max="5000"
        step="100"
        @input="persistSettingsSoon"
      />
    </label>
    <label>扬声器设备
      <select v-model="settingsStore.settings.audioDevices.systemOutputDeviceName" @change="persistSettings">
        <option value="">默认系统输出设备</option>
        <option v-for="device in settingsStore.outputDevices" :key="device.name" :value="device.name">
          {{ device.name }}{{ device.isDefault ? '（默认）' : '' }}
        </option>
      </select>
    </label>
  </section>
</template>
