<script setup lang="ts">
import SystemCaptionDisplay from './SystemCaptionDisplay.vue';
import SystemCaptionHistory from './SystemCaptionHistory.vue';
import SystemSubtitleSettings from './SystemSubtitleSettings.vue';
import StatusBadge from '../shared/StatusBadge.vue';
import AudioLevelMeter from '../shared/AudioLevelMeter.vue';
import ErrorBanner from '../shared/ErrorBanner.vue';
import { useSystemSubtitle } from '../../hooks/system-subtitle/useSystemSubtitle';
import { areLocalServicesReady, localServicesStore } from '../../stores/services/localServicesStore';

const system = useSystemSubtitle();
</script>

<template>
  <article class="panel">
    <header class="panel-header">
      <div>
        <h2>系统字幕翻译</h2>
      </div>
      <StatusBadge :label="system.store.isRunning ? '运行中' : '已停止'" :tone="system.store.isRunning ? 'running' : 'idle'" />
    </header>

    <p class="metric-label">本地服务：{{ localServicesStore.message }}</p>
    <div class="panel-actions">
      <button type="button" :disabled="!areLocalServicesReady() || system.store.isRunning" @click="system.startSystemSubtitle">启动</button>
      <button type="button" class="secondary" :disabled="!system.store.isRunning" @click="system.stopSystemSubtitle">停止</button>
      <button type="button" class="secondary" @click="system.finalizeSystemVisualSegment">手动完成分段</button>
    </div>

    <p class="metric-label">系统音频电平</p>
    <AudioLevelMeter :value="system.store.audioLevel" />
    <ErrorBanner v-if="system.store.errors[0]" :message="system.store.errors[0]" />
    <SystemSubtitleSettings />
    <SystemCaptionDisplay />
    <SystemCaptionHistory />
  </article>
</template>
