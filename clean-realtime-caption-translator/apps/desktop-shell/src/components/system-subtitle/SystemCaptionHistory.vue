<script setup lang="ts">
import { systemSubtitleStore } from '../../stores/system-subtitle/systemSubtitleStore';
import { formatClock } from '../../utils/time';
</script>

<template>
  <section class="history">
    <h3>Caption History</h3>
    <ol>
      <li v-for="segment in systemSubtitleStore.finalizedCaptions" :key="segment.id">
        <time>{{ formatClock(segment.endedAt) }}</time>
        <strong>{{ segment.rawText }}</strong>
        <span>{{ segment.translatedText || '翻译中 / 未配置' }}</span>
      </li>
    </ol>
  </section>
</template>
