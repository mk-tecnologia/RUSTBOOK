<script setup lang="ts">
defineProps<{ message: string; retry?: boolean }>()
defineEmits<{ retry: [] }>()
</script>
<template>
  <div v-if="message" role="alert" class="my-4 rounded-lg border border-red-300 p-4 text-sm text-red-700 bg-red-50 dark:bg-red-950 dark:text-red-200">
    <p>{{ message }}</p>
    <button v-if="retry" class="mt-2 underline font-medium" @click="$emit('retry')">Try again</button>
  </div>
</template>
