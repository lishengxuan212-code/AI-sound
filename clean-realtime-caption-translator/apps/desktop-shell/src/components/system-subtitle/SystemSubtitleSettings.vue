<script setup lang="ts">
import { settingsStore } from '../../stores/settings/settingsStore';
</script>

<template>
  <section class="settings-strip">
    <span>ASR WS: {{ settingsStore.settings?.localAsr.wsUrl ?? 'loading' }}</span>
    <span>Translation: {{ settingsStore.settings?.localTranslation.modelName || '本地翻译模型未配置' }}</span>
  </section>
</template>
