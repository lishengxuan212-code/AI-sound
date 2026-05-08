<script setup lang="ts">
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
</script>

<template>
  <section class="tts-status">
    <h3>TTS</h3>
    <p>Status: {{ micInterpretationStore.ttsStatus }}</p>
    <ol>
      <li v-for="item in micInterpretationStore.ttsQueue.slice(-5)" :key="item.id">
        <span>{{ item.status }}</span>
        <strong>{{ item.text }}</strong>
      </li>
    </ol>
  </section>
</template>
