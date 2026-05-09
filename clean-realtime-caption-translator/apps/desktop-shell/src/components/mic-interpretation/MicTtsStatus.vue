<script setup lang="ts">
import { computed } from 'vue';
import { micInterpretationStore } from '../../stores/mic-interpretation/micInterpretationStore';
import { retryTtsPlayback, stopTtsPlayback } from '../../services/tts/qwenTtsClient';
import { logDiagnostic } from '../../services/diagnostics/diagnosticsLogger';

const latestAudio = computed(() => [...micInterpretationStore.ttsQueue].reverse().find((candidate) => candidate.audioPath));
const statusLabels: Record<string, string> = {
  idle: '已停止',
  queued: '排队中',
  synthesizing: '合成中',
  audio_saved: '音频已保存',
  playing: '播放中',
  completed: '已完成',
  failed: '失败',
};
const statusLabel = computed(() => statusLabels[micInterpretationStore.ttsStatus] ?? micInterpretationStore.ttsStatus);

function retryLatestAudio(): void {
  if (!latestAudio.value) {
    logDiagnostic('[ERROR][TTS_PLAYBACK_FAILED]', { message: '当前没有可重播的 TTS 音频。' });
    return;
  }
  void retryTtsPlayback({ ttsId: latestAudio.value.id });
}

function stopAudio(): void {
  void stopTtsPlayback();
}
</script>

<template>
  <section class="tts-status">
    <h3>语音合成状态</h3>
    <p>当前状态：{{ statusLabel }}</p>
    <p>最近语音：{{ latestAudio?.audioPath || '当前没有可重播的 TTS 音频。' }}</p>
    <div class="panel-actions">
      <button type="button" class="secondary" @click="retryLatestAudio">重播</button>
      <button type="button" class="secondary" @click="stopAudio">停止播放</button>
    </div>
    <ol>
      <li v-for="item in micInterpretationStore.ttsQueue.slice(-5)" :key="item.id">
        <span>{{ statusLabels[item.status] ?? item.status }}</span>
        <strong>{{ item.text }}</strong>
        <small v-if="item.audioPath">音频路径：{{ item.audioPath }}</small>
        <small v-if="item.fileSize">文件大小：{{ item.fileSize }} bytes</small>
        <small v-if="item.error">{{ item.error }}</small>
      </li>
      <li v-if="micInterpretationStore.ttsQueue.length === 0" class="empty-state">暂无 TTS 队列。</li>
    </ol>
  </section>
</template>
