<script setup lang="ts">
import MicTranscriptDisplay from './MicTranscriptDisplay.vue';
import MicTranslationDisplay from './MicTranslationDisplay.vue';
import MicTtsStatus from './MicTtsStatus.vue';
import MicInterpretationSettings from './MicInterpretationSettings.vue';
import StatusBadge from '../shared/StatusBadge.vue';
import AudioLevelMeter from '../shared/AudioLevelMeter.vue';
import ErrorBanner from '../shared/ErrorBanner.vue';
import { useMicInterpretation } from '../../hooks/mic-interpretation/useMicInterpretation';
import { areLocalServicesReady, localServicesStore } from '../../stores/services/localServicesStore';

const mic = useMicInterpretation();
</script>

<template>
  <article class="panel">
    <header class="panel-header">
      <div>
        <h2>麦克风同声传译</h2>
      </div>
      <StatusBadge :label="mic.store.isRunning ? '运行中' : '已停止'" :tone="mic.store.isRunning ? 'running' : 'idle'" />
    </header>

    <p class="metric-label">本地服务：{{ localServicesStore.message }}</p>
    <div class="panel-actions">
      <button type="button" :disabled="!areLocalServicesReady() || mic.store.isRunning" @click="mic.startMicInterpretation">启动</button>
      <button type="button" class="secondary" :disabled="!mic.store.isRunning" @click="mic.stopMicInterpretation">停止</button>
    </div>

    <p class="metric-label">麦克风音频电平</p>
    <AudioLevelMeter :value="mic.store.audioLevel" />
    <ErrorBanner v-if="mic.store.errors[0]" :message="mic.store.errors[0]" />
    <MicInterpretationSettings />
    <MicTranscriptDisplay />
    <MicTranslationDisplay />
    <MicTtsStatus />
  </article>
</template>
