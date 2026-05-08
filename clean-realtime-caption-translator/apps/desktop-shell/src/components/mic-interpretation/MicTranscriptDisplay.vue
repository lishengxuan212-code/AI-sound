<script setup lang="ts">
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
</script>

<template>
  <section class="caption-display">
    <div class="caption-row">
      <span class="label">Mic Raw</span>
      <p>{{ micInterpretationStore.currentRawTranscript || 'Waiting for microphone local ASR...' }}</p>
    </div>
  </section>
</template>
