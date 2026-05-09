<script setup lang="ts">
import { computed } from 'vue';
import { systemSubtitleStore } from '../../stores/system-subtitle/systemSubtitleStore';
import { formatClock } from '../../utils/time';

const historyGroups = computed(() => systemSubtitleStore.stableTranslatedCaptions.slice(0, -1).slice(-3).reverse());
</script>

<template>
  <section class="history">
    <h3>字幕历史</h3>
    <ol>
      <li v-for="segment in historyGroups" :key="segment.id">
        <time>{{ formatClock(segment.endedAt) }}</time>
        <strong>{{ segment.rawText }}</strong>
        <span>{{ segment.translatedText }}</span>
      </li>
      <li v-if="historyGroups.length === 0" class="empty-state">暂无历史字幕。</li>
    </ol>
  </section>
</template>
