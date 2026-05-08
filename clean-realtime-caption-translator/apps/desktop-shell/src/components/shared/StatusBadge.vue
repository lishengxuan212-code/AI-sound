<script setup lang="ts">
defineProps<{ label: string; tone?: 'idle' | 'running' | 'warning' | 'error' }>();
</script>

<template>
  <span class="status-badge" :class="tone ?? 'idle'">{{ label }}</span>
</template>
