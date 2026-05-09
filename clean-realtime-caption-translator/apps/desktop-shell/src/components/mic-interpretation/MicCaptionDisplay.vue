<script setup lang="ts">
import { computed } from 'vue';
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';

const rawText = computed(() => micInterpretationStore.currentRawTranscript.trim());
const translatedText = computed(() => micInterpretationStore.currentTranslatedText.trim());
const hasCaption = computed(() => Boolean(rawText.value || translatedText.value));
const statusLabel = computed(() => {
  if (micInterpretationStore.ttsStatus === 'failed') return '传译失败';
  if (translatedText.value && ['playing', 'completed'].includes(micInterpretationStore.ttsStatus)) return '已传译';
  if (rawText.value || ['queued', 'synthesizing', 'audio_saved'].includes(micInterpretationStore.ttsStatus)) return '传译中';
  return '待机';
});
</script>

<template>
  <section class="caption-display single-caption-display mic-caption-display">
    <div v-if="hasCaption" class="single-caption-group">
      <div class="mic-caption-status">{{ statusLabel }}</div>
      <div class="caption-lines mic-caption-lines">
        <span class="caption-line">{{ rawText }}</span>
      </div>
      <p class="caption-translation">{{ translatedText }}</p>
    </div>
    <div v-else class="caption-empty">
      <span class="label">麦克风同声传译</span>
      <p>等待麦克风识别结果。</p>
    </div>
  </section>
</template>
