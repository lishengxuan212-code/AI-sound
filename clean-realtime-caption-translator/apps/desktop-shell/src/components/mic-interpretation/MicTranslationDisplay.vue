<script setup lang="ts">
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
</script>

<template>
  <section class="caption-display">
    <div class="caption-row translated">
      <span class="label">Mic Translated</span>
      <p>{{ micInterpretationStore.currentTranslatedText || '本地翻译模型未配置或等待翻译结果' }}</p>
    </div>
  </section>
</template>
