<script setup lang="ts">
import { computed, onMounted } from 'vue';
import LogsPanel from '../logs/LogsPanel.vue';
import { settingsStore } from '../../stores/settings/settingsStore';
import {
  getSecretStatus,
  listAudioDevices,
  listTtsVoices,
  resetAppSettings,
  saveAppSettings,
  testLocalAsrConnection,
  testLocalTranslationConnection,
  testTtsConnection,
} from '../../services/settings/settingsManager';
import { logDiagnostic } from '../../services/diagnostics/diagnosticsLogger';

const CLOUD_PROVIDER = 'dashscope_openai';
const LOCAL_PROVIDER = 'transformers_seq2seq';
const LOCAL_TRANSLATION_ENDPOINT = 'http://127.0.0.1:8777';
const CLOUD_TRANSLATION_ENDPOINT = 'https://ark.cn-beijing.volces.com/api/v3/chat/completions';

const settings = computed(() => settingsStore.settings);
const isCloudTranslation = computed(() => settings.value?.localTranslation.provider === CLOUD_PROVIDER);
const isLocalTranslation = computed(() => settings.value?.localTranslation.provider === LOCAL_PROVIDER);
let persistTimer: number | undefined;

async function refreshOptions(): Promise<void> {
  const [devices, voices] = await Promise.all([listAudioDevices(), listTtsVoices()]);
  settingsStore.inputDevices = devices.inputDevices;
  settingsStore.outputDevices = devices.outputDevices;
  settingsStore.ttsVoices = voices;
}

onMounted(() => {
  void refreshOptions();
});

async function persistSettings(showMessage = false): Promise<void> {
  if (!settingsStore.settings) return;
  settingsStore.saving = true;
  try {
    settingsStore.settings = await saveAppSettings(settingsStore.settings);
    if (showMessage) {
      settingsStore.lastMessage = '设置已保存到本地。';
      logDiagnostic('[SETTINGS][SAVE]', { status: 'success' });
    }
  } catch (error) {
    settingsStore.lastMessage = `设置保存失败：${error instanceof Error ? error.message : String(error)}`;
    logDiagnostic('[SETTINGS][SAVE]', { status: 'failed', error: settingsStore.lastMessage });
  } finally {
    settingsStore.saving = false;
  }
}

function persistSettingsSoon(): void {
  if (persistTimer) window.clearTimeout(persistTimer);
  persistTimer = window.setTimeout(() => {
    persistTimer = undefined;
    void persistSettings();
  }, 400);
}

async function saveSettings(): Promise<void> {
  await persistSettings(true);
}

async function resetSettings(): Promise<void> {
  settingsStore.settings = await resetAppSettings();
  settingsStore.lastMessage = '设置已恢复默认值。';
  await refreshOptions();
  logDiagnostic('[SETTINGS][RESET]', { status: 'success' });
}

async function refreshSecretStatus(): Promise<void> {
  if (!settingsStore.settings) return;
  const status = await getSecretStatus();
  settingsStore.settings.qwenTts.gameTtsApiKeyConfigured = status.gameTtsApiKeyConfigured;
  settingsStore.settings.qwenTts.dashscopeApiKeyConfigured = status.dashscopeApiKeyConfigured;
  settingsStore.settings.qwenTts.hasApiKey =
    Boolean(settingsStore.settings.qwenTts.apiKey) || status.gameTtsApiKeyConfigured || status.dashscopeApiKeyConfigured;
}

function changeTranslationProvider(): void {
  if (!settingsStore.settings) return;
  const translation = settingsStore.settings.localTranslation;
  if (translation.provider === LOCAL_PROVIDER) {
    translation.endpoint = LOCAL_TRANSLATION_ENDPOINT;
    if (!translation.modelName || translation.modelName.startsWith('ep-')) {
      translation.modelName = 'Helsinki-NLP/opus-mt-en-zh';
    }
  } else if (translation.provider === CLOUD_PROVIDER) {
    if (!translation.endpoint || translation.endpoint.includes('127.0.0.1')) {
      translation.endpoint = CLOUD_TRANSLATION_ENDPOINT;
    }
  }
  void persistSettings();
}

async function runAsrTest(): Promise<void> {
  const result = await testLocalAsrConnection();
  settingsStore.lastMessage = `本地 ASR：${result.message}`;
  logDiagnostic(result.ok ? '[SETTINGS][TEST_ASR]' : '[ERROR][LOCAL_ASR_NOT_CONNECTED]', result);
}

async function runTranslationTest(): Promise<void> {
  await persistSettings();
  const result = await testLocalTranslationConnection();
  settingsStore.lastMessage = `翻译配置：${result.message}`;
  const prefix = result.ok
    ? '[SETTINGS][TEST_TRANSLATION]'
    : settingsStore.settings?.localTranslation.endpoint
      ? '[ERROR][LOCAL_TRANSLATION_SERVER_UNREACHABLE]'
      : '[ERROR][LOCAL_TRANSLATION_ENDPOINT_MISSING]';
  logDiagnostic(prefix, result);
}

async function runTtsTest(): Promise<void> {
  await persistSettings();
  await refreshSecretStatus();
  const result = await testTtsConnection();
  settingsStore.lastMessage = `TTS：${result.message}`;
  logDiagnostic(result.ok ? '[SETTINGS][TEST_TTS]' : '[ERROR][TTS_CONFIG_INVALID]', result);
}
</script>

<template>
  <section v-if="settings" class="settings-stack">
    <article class="panel settings-panel">
      <header class="panel-header">
        <div>
          <h2>设置</h2>
          <p>本地 ASR、翻译模型、TTS 和日志配置。</p>
        </div>
        <div class="panel-actions compact-actions">
          <button type="button" :disabled="settingsStore.saving" @click="saveSettings">保存</button>
          <button type="button" class="secondary" @click="resetSettings">重置</button>
        </div>
      </header>

      <p v-if="settingsStore.lastMessage" class="notice">{{ settingsStore.lastMessage }}</p>

      <section class="settings-grid">
        <fieldset>
          <legend>本地 ASR</legend>
          <label>识别引擎 <input v-model="settings.localAsr.provider" /></label>
          <label>ASR WebSocket <input v-model="settings.localAsr.wsUrl" /></label>
          <label>模型目录 <input v-model="settings.localAsr.modelDir" /></label>
          <label>采样率 <input v-model.number="settings.localAsr.sampleRate" type="number" min="1" /></label>
          <label>语言模式 <input v-model="settings.localAsr.languageMode" /></label>
          <label class="check-row">
            <input v-model="settings.localAsr.enableEndpointDetection" type="checkbox" /> 启用 endpoint detection
          </label>
          <button type="button" class="secondary" @click="runAsrTest">测试本地 ASR 连接</button>
        </fieldset>

        <fieldset>
          <legend>翻译模型</legend>
          <label>
            启用引擎
            <select v-model="settings.localTranslation.provider" @change="changeTranslationProvider">
              <option :value="LOCAL_PROVIDER">本地模型 opus-mt-en-zh</option>
              <option :value="CLOUD_PROVIDER">云端模型（OpenAI 兼容接口）</option>
            </select>
          </label>

          <template v-if="isLocalTranslation">
            <label>本地服务地址 <input v-model="settings.localTranslation.endpoint" @change="persistSettings()" /></label>
            <label>本地模型名称 <input v-model="settings.localTranslation.modelName" placeholder="Helsinki-NLP/opus-mt-en-zh" @change="persistSettings()" /></label>
            <label>本地模型路径 <input v-model="settings.localTranslation.modelPath" placeholder="可选，留空时按模型名称加载" @change="persistSettings()" /></label>
          </template>

          <template v-if="isCloudTranslation">
            <label>云端 API 地址 <input v-model="settings.localTranslation.endpoint" @change="persistSettings()" /></label>
            <label>云端 API Key <input v-model="settings.localTranslation.apiKey" type="password" autocomplete="off" @change="persistSettings()" /></label>
            <label>云端模型名称 <input v-model="settings.localTranslation.modelName" placeholder="例如 ep-xxx 或 qwen-plus" @change="persistSettings()" /></label>
            <label>代理地址（可选） <input v-model="settings.localTranslation.proxyUrl" placeholder="例如 http://127.0.0.1:7890" @change="persistSettings()" /></label>
            <label>
              云端翻译提示词
              <textarea
                v-model="settings.localTranslation.prompt"
                rows="4"
                placeholder="例如：翻译为自然口语化中文，保留游戏术语，不要解释。"
                @input="persistSettingsSoon"
              />
            </label>
          </template>

          <label>超时时间(ms) <input v-model.number="settings.localTranslation.timeoutMs" type="number" min="1" @change="persistSettings()" /></label>
          <button type="button" class="secondary" @click="runTranslationTest">测试翻译配置</button>
        </fieldset>

        <fieldset>
          <legend>TTS API</legend>
          <label>API 地址 <input v-model="settings.qwenTts.endpoint" @change="persistSettings()" /></label>
          <label>API Key
            <input v-model="settings.qwenTts.apiKey" type="password" autocomplete="off" @change="persistSettings()" />
          </label>
          <label>模型名称 <input v-model="settings.qwenTts.model" @change="persistSettings()" /></label>
          <label>音频格式
            <select v-model="settings.qwenTts.format" @change="persistSettings()">
              <option value="wav">wav</option>
              <option value="mp3">mp3</option>
              <option value="pcm">pcm</option>
            </select>
          </label>
          <label>音色
            <select v-model="settings.qwenTts.voice" @change="persistSettings()">
              <option value="">请选择音色</option>
              <option v-for="voice in settingsStore.ttsVoices" :key="voice.value" :value="voice.value">
                {{ voice.labelZh }}
              </option>
            </select>
          </label>
          <label>采样率 <input v-model.number="settings.qwenTts.sampleRate" type="number" min="1" @change="persistSettings()" /></label>
          <label>语速 <input v-model.number="settings.qwenTts.speed" type="number" step="0.1" @change="persistSettings()" /></label>
          <label>音量 <input v-model.number="settings.qwenTts.volume" type="number" min="0" max="100" @change="persistSettings()" /></label>
          <button type="button" class="secondary" @click="runTtsTest">测试 TTS 配置</button>
        </fieldset>

        <fieldset>
          <legend>日志</legend>
          <label class="check-row">
            <input v-model="settings.diagnostics.diagnosticsEnabled" type="checkbox" @change="persistSettings()" /> 开启日志上报
          </label>
          <label>日志级别
            <select v-model="settings.diagnostics.logLevel" @change="persistSettings()">
              <option value="debug">debug</option>
              <option value="info">info</option>
              <option value="warn">warn</option>
              <option value="error">error</option>
            </select>
          </label>
          <label class="check-row">
            <input v-model="settings.diagnostics.showSystemLogs" type="checkbox" @change="persistSettings()" /> 系统字幕日志
          </label>
          <label class="check-row">
            <input v-model="settings.diagnostics.showMicLogs" type="checkbox" @change="persistSettings()" /> 麦克风同传日志
          </label>
          <label class="check-row">
            <input v-model="settings.diagnostics.showAsrLogs" type="checkbox" @change="persistSettings()" /> ASR 日志
          </label>
          <label class="check-row">
            <input v-model="settings.diagnostics.showTranslationLogs" type="checkbox" @change="persistSettings()" /> 翻译日志
          </label>
          <label class="check-row">
            <input v-model="settings.diagnostics.showTtsLogs" type="checkbox" @change="persistSettings()" /> TTS 日志
          </label>
          <label class="check-row">
            <input v-model="settings.diagnostics.showErrorLogs" type="checkbox" @change="persistSettings()" /> 错误日志
          </label>
        </fieldset>
      </section>
    </article>

    <LogsPanel v-if="settings.diagnostics.diagnosticsEnabled" />
  </section>
</template>
