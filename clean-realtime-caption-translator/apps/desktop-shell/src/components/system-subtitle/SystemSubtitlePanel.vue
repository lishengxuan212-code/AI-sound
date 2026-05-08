<script setup lang="ts">
import SystemCaptionDisplay from './SystemCaptionDisplay.vue';
import SystemCaptionHistory from './SystemCaptionHistory.vue';
import SystemSubtitleSettings from './SystemSubtitleSettings.vue';
import StatusBadge from '../shared/StatusBadge.vue';
import AudioLevelMeter from '../shared/AudioLevelMeter.vue';
import ErrorBanner from '../shared/ErrorBanner.vue';
import { useSystemSubtitle } from '../../hooks/system-subtitle/useSystemSubtitle';

const system = useSystemSubtitle();
</script>

<template>
  <article class="panel">
    <header class="panel-header">
      <div>
        <h2>System Subtitle</h2>
        <p>System audio captions and local translation.</p>
      </div>
      <StatusBadge :label="system.store.isRunning ? 'Running' : 'Idle'" :tone="system.store.isRunning ? 'running' : 'idle'" />
    </header>

    <div class="panel-actions">
      <button type="button" @click="system.startSystemSubtitle">Start</button>
      <button type="button" class="secondary" @click="system.stopSystemSubtitle">Stop</button>
      <button type="button" class="secondary" @click="system.finalizeSystemVisualSegment">Finalize Segment</button>
    </div>

    <AudioLevelMeter :value="system.store.audioLevel" />
    <ErrorBanner v-if="system.store.errors[0]" :message="system.store.errors[0]" />
    <SystemCaptionDisplay />
    <SystemSubtitleSettings />
    <SystemCaptionHistory />
  </article>
</template>
