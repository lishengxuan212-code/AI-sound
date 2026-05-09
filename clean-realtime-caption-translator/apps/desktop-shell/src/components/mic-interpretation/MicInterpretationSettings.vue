<script setup lang="ts">
import { settingsStore } from '../../stores/settings/settingsStore';
import { saveAppSettings } from '../../services/settings/settingsManager';
import { useMicInterpretation } from '../../hooks/mic-interpretation/useMicInterpretation';

const EN_ASR_MODEL_DIR = 'local-asr\\models\\vosk-model-small-en-us-0.15';
const ZH_ASR_MODEL_DIR = 'local-asr\\models\\sherpa-onnx-streaming-paraformer-bilingual-zh-en';

const mic = useMicInterpretation();
const languageDirections = [
  { label: '中文 -> 英文', source: 'zh', target: 'en', asrModelDir: ZH_ASR_MODEL_DIR },
  { label: '英文 -> 中文', source: 'en', target: 'zh', asrModelDir: EN_ASR_MODEL_DIR },
];

let restarting = false;

async function persistSettings(): Promise<void> {
  if (!settingsStore.settings) return;
  settingsStore.settings = await saveAppSettings(settingsStore.settings);
}

async function changeDirection(event: Event): Promise<void> {
  if (!settingsStore.settings || restarting) return;
  const value = String((event.target as HTMLSelectElement).value);
  const direction = languageDirections.find((item) => `${item.source}->${item.target}` === value);
  if (!direction) return;

  const wasRunning = mic.store.isRunning;
  restarting = true;
  try {
    settingsStore.settings.language.micSourceLang = direction.source;
    settingsStore.settings.language.micTargetLang = direction.target;
    settingsStore.settings.localAsr.provider = 'vosk';
    settingsStore.settings.localAsr.modelDir = direction.asrModelDir;
    await persistSettings();

    if (wasRunning) {
      await mic.stopMicInterpretation();
      await mic.startMicInterpretation();
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
        :value="`${settingsStore.settings.language.micSourceLang}->${settingsStore.settings.language.micTargetLang}`"
        :disabled="restarting"
        @change="changeDirection"
      >
        <option v-for="item in languageDirections" :key="`${item.source}-${item.target}`" :value="`${item.source}->${item.target}`">
          {{ item.label }}
        </option>
      </select>
    </label>
    <label>麦克风设备
      <select v-model="settingsStore.settings.audioDevices.micInputDeviceName" @change="persistSettings">
        <option value="">默认麦克风输入设备</option>
        <option v-for="device in settingsStore.inputDevices" :key="device.name" :value="device.name">
          {{ device.name }}{{ device.isDefault ? '（默认）' : '' }}
        </option>
      </select>
    </label>
  </section>
</template>
