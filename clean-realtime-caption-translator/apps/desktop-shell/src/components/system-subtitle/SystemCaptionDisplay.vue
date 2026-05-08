<script setup lang="ts">
import { systemSubtitleStore } from '../../stores/system-subtitle/systemSubtitleStore';
</script>

<template>
  <section class="caption-display">
    <div class="caption-row">
      <span class="label">Raw</span>
      <p>{{ systemSubtitleStore.currentRawCaption || 'Waiting for local ASR partials...' }}</p>
    </div>
    <div class="caption-row translated">
      <span class="label">Translated</span>
      <p>{{ systemSubtitleStore.currentTranslatedCaption || '本地翻译模型未配置或等待翻译结果' }}</p>
    </div>
  </section>
</template>
