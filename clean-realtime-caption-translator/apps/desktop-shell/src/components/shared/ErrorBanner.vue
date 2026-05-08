<script setup lang="ts">
defineProps<{ message: string }>();
</script>

<template>
  <div class="error-banner">{{ message }}</div>
</template>
