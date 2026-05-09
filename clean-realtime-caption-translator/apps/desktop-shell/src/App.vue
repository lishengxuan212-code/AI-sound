<script setup lang="ts">
import { ref } from 'vue';
import SystemSubtitlePanel from './components/system-subtitle/SystemSubtitlePanel.vue';
import MicInterpretationPanel from './components/mic-interpretation/MicInterpretationPanel.vue';
import SettingsPanel from './components/settings/SettingsPanel.vue';
import ErrorBanner from './components/shared/ErrorBanner.vue';
import { dismissTip, settingsStore } from './stores/settings/settingsStore';

const activePage = ref<'home' | 'settings'>('home');
</script>

<template>
  <main class="app-shell">
    <header class="app-header">
      <div>
        <h1>本地实时字幕与同声传译</h1>
      </div>
      <div class="panel-actions compact-actions">
        <button v-if="activePage === 'settings'" type="button" class="secondary" @click="activePage = 'home'">返回首页</button>
        <button v-else type="button" class="secondary" @click="activePage = 'settings'">设置</button>
      </div>
    </header>

    <ErrorBanner v-if="settingsStore.error" :message="settingsStore.error" />
    <div v-if="settingsStore.tips.length" class="tips-stack">
      <div v-for="(tip, index) in settingsStore.tips" :key="`${tip}-${index}`" class="tip">
        <span>{{ tip }}</span>
        <button type="button" class="secondary" @click="dismissTip(index)">关闭</button>
      </div>
    </div>

    <section v-if="activePage === 'home'" class="workspace-grid">
      <SystemSubtitlePanel />
      <MicInterpretationPanel />
    </section>

    <SettingsPanel v-else />
  </main>
</template>
