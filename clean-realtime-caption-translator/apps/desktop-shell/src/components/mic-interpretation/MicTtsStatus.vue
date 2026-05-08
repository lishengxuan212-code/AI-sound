<script setup lang="ts">
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
import { retryTtsPlayback } from '../../services/tts/qwenTtsClient';

function retryLatestAudio(): void {
  const item = [...micInterpretationStore.ttsQueue].reverse().find((candidate) => candidate.audioPath);
  if (!item?.audioPath) return;
  void retryTtsPlayback({
    ttsId: item.id,
    translationItemId: item.translationItemId,
    audioPath: item.audioPath,
  });
}
</script>

<template>
  <section class="tts-status">
    <h3>TTS</h3>
    <p>Status: {{ micInterpretationStore.ttsStatus }}</p>
    <button type="button" @click="retryLatestAudio">Retry last audio</button>
    <ol>
      <li v-for="item in micInterpretationStore.ttsQueue.slice(-5)" :key="item.id">
        <span>{{ item.status }}</span>
        <strong>{{ item.text }}</strong>
        <small v-if="item.audioPath">{{ item.audioPath }}</small>
      </li>
    </ol>
  </section>
</template>
