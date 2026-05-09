<script setup lang="ts">
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
</script>

<template>
  <section class="caption-row">
    <span class="label">麦克风识别原文</span>
    <p>{{ micInterpretationStore.currentRawTranscript || '等待麦克风本地 ASR 识别结果。' }}</p>
  </section>
</template>
