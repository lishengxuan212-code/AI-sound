<script setup lang="ts">
import { settingsStore } from '../../stores/settings/settingsStore';
</script>

<template>
  <section class="settings-strip">
    <span>TTS model: {{ settingsStore.settings?.qwenTts.model ?? 'qwen-qwen-tts-latest' }}</span>
    <span>Key present: {{ settingsStore.settings?.qwenTts.hasApiKey ? 'true' : 'false' }}</span>
  </section>
</template>
