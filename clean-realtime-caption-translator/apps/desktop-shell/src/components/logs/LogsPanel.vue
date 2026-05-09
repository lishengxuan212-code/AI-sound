<script setup lang="ts">
import { computed } from 'vue';
import { clearLogs, exportLogs, filteredLogs, levelLabels, logsStore, moduleLabels, type LogModule } from '../../stores/logs/logsStore';

const selectedLog = computed(() => logsStore.entries.find((entry) => entry.id === logsStore.selectedId));
const moduleOptions: Array<{ value: LogModule | 'all' | 'error'; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'system', label: '系统字幕' },
  { value: 'mic', label: '麦克风同声传译' },
  { value: 'asr', label: '本地 ASR' },
  { value: 'translation', label: '本地翻译' },
  { value: 'tts', label: 'TTS' },
  { value: 'settings', label: '设置' },
  { value: 'error', label: '错误' },
];

function confirmClear(): void {
  if (window.confirm('确定清空当前 UI 日志吗？')) clearLogs();
}
</script>

<template>
  <article class="panel logs-panel">
    <header class="panel-header">
      <div>
        <h2>日志</h2>
        <p>仅在开启日志上报后记录，内容会自动脱敏。</p>
      </div>
      <div class="panel-actions compact-actions">
        <button type="button" class="secondary" @click="confirmClear">清空日志</button>
        <button type="button" class="secondary" @click="exportLogs('json')">导出 JSON</button>
        <button type="button" class="secondary" @click="exportLogs('txt')">导出 TXT</button>
      </div>
    </header>

    <div class="log-toolbar">
      <label>筛选
        <select v-model="logsStore.moduleFilter">
          <option v-for="option in moduleOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
      </label>
      <label>日志级别
        <select v-model="logsStore.levelFilter">
          <option value="all">全部</option>
          <option value="debug">调试</option>
          <option value="info">信息</option>
          <option value="warn">警告</option>
          <option value="error">错误</option>
        </select>
      </label>
      <label>搜索 <input v-model="logsStore.search" placeholder="message / eventType / sessionId / ttsId" /></label>
    </div>

    <section class="log-layout">
      <ol class="log-list">
        <li
          v-for="entry in filteredLogs"
          :key="entry.id"
          :class="['log-entry', entry.level, { selected: entry.id === logsStore.selectedId }]"
          @click="logsStore.selectedId = entry.id"
        >
          <time>{{ new Date(entry.timestamp).toLocaleTimeString('zh-CN') }}</time>
          <strong>{{ levelLabels[entry.level] }}</strong>
          <span>{{ moduleLabels[entry.module] }}</span>
          <code>{{ entry.eventType }}</code>
          <p>{{ entry.message }}</p>
        </li>
        <li v-if="filteredLogs.length === 0" class="empty-state">暂无日志。</li>
      </ol>

      <aside class="log-detail">
        <h3>日志详情</h3>
        <template v-if="selectedLog">
          <dl>
            <dt>时间</dt>
            <dd>{{ new Date(selectedLog.timestamp).toLocaleString('zh-CN') }}</dd>
            <dt>级别</dt>
            <dd>{{ levelLabels[selectedLog.level] }}</dd>
            <dt>模块</dt>
            <dd>{{ moduleLabels[selectedLog.module] }}</dd>
            <dt>sessionKind</dt>
            <dd>{{ selectedLog.sessionKind ?? '无' }}</dd>
            <dt>eventType</dt>
            <dd>{{ selectedLog.eventType }}</dd>
            <dt>message</dt>
            <dd>{{ selectedLog.message }}</dd>
            <dt>已脱敏</dt>
            <dd>{{ selectedLog.redacted ? '是' : '否' }}</dd>
          </dl>
          <pre>{{ JSON.stringify(selectedLog.details ?? {}, null, 2) }}</pre>
        </template>
        <p v-else class="empty-state">请选择一条日志查看详情。</p>
      </aside>
    </section>
  </article>
</template>
