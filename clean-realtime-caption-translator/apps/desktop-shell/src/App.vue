<script setup lang="ts">
import SystemSubtitlePanel from './components/system-subtitle/SystemSubtitlePanel.vue';
import MicInterpretationPanel from './components/mic-interpretation/MicInterpretationPanel.vue';
import ErrorBanner from './components/shared/ErrorBanner.vue';
import { settingsStore } from './stores/settings/settingsStore';
</script>

<template>
  <main class="app-shell">
    <header class="app-header">
      <div>
        <h1>AI Sound Realtime Translator</h1>
        <p>Local ASR and local translation for captions, Qwen TTS for microphone interpretation.</p>
      </div>
      <div class="header-meta">
        <span>ASR: local sherpa/vosk</span>
        <span>TTS: qwen-qwen-tts-latest</span>
      </div>
    </header>

    <ErrorBanner v-if="settingsStore.error" :message="settingsStore.error" />

    <section class="workspace-grid">
      <SystemSubtitlePanel />
      <MicInterpretationPanel />
    </section>
  </main>
</template>
