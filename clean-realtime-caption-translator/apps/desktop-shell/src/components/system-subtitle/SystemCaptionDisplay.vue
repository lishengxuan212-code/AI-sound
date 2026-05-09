<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { systemSubtitleStore } from '../../stores/system-subtitle/systemSubtitleStore';

const SOURCE_LINE_CHAR_LIMIT = 64;
const SOURCE_MAX_LINES = 3;
const EMPTY_HOLD_MS = 700;

interface CaptionLine {
  key: string;
  text: string;
}

const latestGroup = computed(() => systemSubtitleStore.stableTranslatedCaptions.at(-1));
const stableSourceText = computed(() => normalizeSource(latestGroup.value?.rawText || ''));
const stableTranslationText = computed(() => latestGroup.value?.translatedText || '');
const draftSourceText = computed(() => normalizeSource(systemSubtitleStore.currentRawCaption || ''));
const sourceBufferText = ref('');
const visibleSourceLines = computed(() => wrapSourceLines(sourceBufferText.value).slice(-SOURCE_MAX_LINES));
const hasCaption = computed(() => visibleSourceLines.value.length > 0 || Boolean(stableTranslationText.value));
let emptyHoldTimer: number | undefined;

watch(
  [draftSourceText, stableSourceText],
  ([draft, stable]) => {
    const next = draft || stable;
    if (next) {
      if (emptyHoldTimer) {
        window.clearTimeout(emptyHoldTimer);
        emptyHoldTimer = undefined;
      }
      sourceBufferText.value = next;
      return;
    }
    holdBeforeEmpty();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (emptyHoldTimer) window.clearTimeout(emptyHoldTimer);
});

function holdBeforeEmpty(): void {
  if (!sourceBufferText.value || emptyHoldTimer) return;
  emptyHoldTimer = window.setTimeout(() => {
    emptyHoldTimer = undefined;
    if (!draftSourceText.value && !stableSourceText.value) {
      sourceBufferText.value = '';
    }
  }, EMPTY_HOLD_MS);
}

function normalizeSource(value: string): string {
  const compacted = value.replace(/\s+/g, ' ').trim();
  return removeRepeatedWholeText(compacted);
}

function removeRepeatedWholeText(value: string): string {
  const words = value.split(' ').filter(Boolean);
  if (words.length < 2 || words.length % 2 !== 0) return value;
  const half = words.length / 2;
  const first = words.slice(0, half).join(' ').toLowerCase();
  const second = words.slice(half).join(' ').toLowerCase();
  return first === second ? words.slice(0, half).join(' ') : value;
}

function wrapSourceLines(text: string): CaptionLine[] {
  const words = text.split(' ').filter(Boolean);
  if (words.length <= 1) return wrapContinuousText(text);
  const lines: CaptionLine[] = [];
  let line = '';
  let startWord = 0;

  words.forEach((word, index) => {
    const nextLine = line ? `${line} ${word}` : word;
    if (line && nextLine.length > SOURCE_LINE_CHAR_LIMIT) {
      lines.push({ key: `${startWord}-${index}-${line}`, text: line });
      line = word;
      startWord = index;
      return;
    }
    line = nextLine;
  });

  if (line) {
    lines.push({ key: `${startWord}-${words.length}-${line}`, text: line });
  }
  return lines;
}

function wrapContinuousText(text: string): CaptionLine[] {
  const lines: CaptionLine[] = [];
  for (let index = 0; index < text.length; index += SOURCE_LINE_CHAR_LIMIT) {
    const line = text.slice(index, index + SOURCE_LINE_CHAR_LIMIT);
    if (line) lines.push({ key: `${index}-${line}`, text: line });
  }
  return lines;
}
</script>

<template>
  <section class="caption-display single-caption-display">
    <div v-if="hasCaption" class="single-caption-group">
      <div class="caption-lines">
        <span v-for="line in visibleSourceLines" :key="line.key" class="caption-line">{{ line.text }}</span>
      </div>
      <p class="caption-translation">{{ stableTranslationText }}</p>
    </div>
    <div v-else class="caption-empty">
      <span class="label">双语字幕</span>
      <p>等待本地 ASR 识别结果。</p>
    </div>
  </section>
</template>
