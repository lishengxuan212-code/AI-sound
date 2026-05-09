<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { systemSubtitleStore } from '../../stores/system-subtitle/systemSubtitleStore';

const SOURCE_EN_LINE_CHAR_LIMIT = 64;
const SOURCE_CJK_LINE_CHAR_LIMIT = 36;
const SOURCE_MAX_LINES = 3;
const TRANSLATION_LINE_CHAR_LIMIT = 62;
const TRANSLATION_MAX_LINES = 3;
const EMPTY_HOLD_MS = 700;

interface CaptionLine {
  key: string;
  text: string;
}

const latestGroup = computed(() => systemSubtitleStore.stableTranslatedCaptions.at(-1));
const stableSourceText = computed(() => normalizeSource(latestGroup.value?.rawText || ''));
const stableTranslationText = computed(() => normalizeTranslation(latestGroup.value?.translatedText || ''));
const draftSourceText = computed(() => normalizeSource(systemSubtitleStore.currentRawCaption || ''));
const sourceBufferText = ref('');
const visibleSourceLines = computed(() => wrapSourceLines(sourceBufferText.value).slice(-SOURCE_MAX_LINES));
const visibleTranslationLines = computed(() => wrapTextLines(stableTranslationText.value, TRANSLATION_LINE_CHAR_LIMIT).slice(-TRANSLATION_MAX_LINES));
const hasCaption = computed(() => visibleSourceLines.value.length > 0 || visibleTranslationLines.value.length > 0);
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

function normalizeTranslation(value: string): string {
  const compacted = value.replace(/\s+/g, ' ').trim();
  return removeRepeatedSentenceRuns(removeRepeatedWholeText(compacted));
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
  return hasCjk(text)
    ? wrapContinuousText(text.replace(/\s+/g, ''), SOURCE_CJK_LINE_CHAR_LIMIT)
    : wrapTextLines(text, SOURCE_EN_LINE_CHAR_LIMIT);
}

function wrapTextLines(text: string, limit: number): CaptionLine[] {
  const words = text.split(' ').filter(Boolean);
  if (words.length <= 1) return wrapContinuousText(text, limit);
  const lines: CaptionLine[] = [];
  let line = '';
  let startWord = 0;

  words.forEach((word, index) => {
    const nextLine = line ? `${line} ${word}` : word;
    if (line && nextLine.length > limit) {
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

function hasCjk(text: string): boolean {
  return /[\u3400-\u9fff]/.test(text);
}

function wrapContinuousText(text: string, limit: number): CaptionLine[] {
  const lines: CaptionLine[] = [];
  for (let index = 0; index < text.length; index += limit) {
    const line = text.slice(index, index + limit);
    if (line) lines.push({ key: `${index}-${line}`, text: line });
  }
  return lines;
}

function removeRepeatedSentenceRuns(value: string): string {
  const sentences = value.match(/[^.!?。！？]+[.!?。！？]?/g)?.map((item) => item.trim()).filter(Boolean) ?? [];
  if (sentences.length < 2) return value;

  const result: string[] = [];
  for (const sentence of sentences) {
    const normalized = sentence.toLowerCase();
    const recent = result.slice(-Math.min(result.length, 8)).map((item) => item.toLowerCase());
    if (recent.includes(normalized)) continue;
    result.push(sentence);
  }
  return result.join(' ');
}
</script>

<template>
  <section class="caption-display single-caption-display">
    <div v-if="hasCaption" class="single-caption-group">
      <div class="caption-lines">
        <span v-for="line in visibleSourceLines" :key="line.key" class="caption-line">{{ line.text }}</span>
      </div>
      <div class="caption-translation-lines">
        <span v-for="line in visibleTranslationLines" :key="line.key" class="caption-translation-line">{{ line.text }}</span>
      </div>
    </div>
    <div v-else class="caption-empty">
      <span class="label">双语字幕</span>
      <p>等待本地 ASR 识别结果。</p>
    </div>
  </section>
</template>
