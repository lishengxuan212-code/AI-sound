<script setup lang="ts">
defineProps<{ value: number }>();
</script>

<template>
  <div class="meter" aria-label="Audio level">
    <div class="meter-fill" :style="{ width: `${Math.max(0, Math.min(100, value * 100))}%` }" />
  </div>
</template>
