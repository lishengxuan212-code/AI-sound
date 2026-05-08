<script setup lang="ts">
import MicTranscriptDisplay from './MicTranscriptDisplay.vue';
import MicTranslationDisplay from './MicTranslationDisplay.vue';
import MicTtsStatus from './MicTtsStatus.vue';
import MicInterpretationSettings from './MicInterpretationSettings.vue';
import StatusBadge from '../shared/StatusBadge.vue';
import AudioLevelMeter from '../shared/AudioLevelMeter.vue';
import { useMicInterpretation } from '../../hooks/mic-interpretation/useMicInterpretation';

const mic = useMicInterpretation();
</script>

<template>
  <article class="panel">
    <header class="panel-header">
      <div>
        <h2>Mic Interpretation</h2>
        <p>Microphone ASR, local translation, Qwen TTS playback.</p>
      </div>
      <StatusBadge :label="mic.store.isRunning ? 'Running' : 'Idle'" :tone="mic.store.isRunning ? 'running' : 'idle'" />
    </header>

    <div class="panel-actions">
      <button type="button" @click="mic.startMicInterpretation">Start</button>
      <button type="button" class="secondary" @click="mic.stopMicInterpretation">Stop</button>
    </div>

    <AudioLevelMeter :value="mic.store.audioLevel" />
    <MicTranscriptDisplay />
    <MicTranslationDisplay />
    <MicTtsStatus />
    <MicInterpretationSettings />
  </article>
</template>
