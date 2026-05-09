<script setup lang="ts">
import { computed } from 'vue';
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
import { formatClock } from '../../utils/time';

const historyItems = computed(() => micInterpretationStore.history.slice(0, 8));
</script>

<template>
  <section class="history mic-history">
    <h3>传译历史</h3>
    <ol>
      <li v-for="item in historyItems" :key="`${item.createdAt}-${item.rawText}`">
        <time>{{ formatClock(item.createdAt) }}</time>
        <strong>{{ item.rawText }}</strong>
        <span>{{ item.translatedText }}</span>
      </li>
      <li v-if="historyItems.length === 0" class="empty-state">暂无传译历史。</li>
    </ol>
  </section>
</template>
