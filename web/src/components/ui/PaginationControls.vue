<script setup lang="ts">
import { computed } from 'vue'
const props = defineProps<{ page: number; pageSize: number; total: number; loading?: boolean }>()
const emit = defineEmits<{ change: [page: number] }>()
const pages = computed(() => Math.max(1, Math.ceil(props.total / props.pageSize)))
</script>

<template>
  <nav aria-label="Pagination" class="flex flex-wrap items-center justify-between gap-3 mt-6 text-sm text-gray-600 dark:text-rd-text-secondary">
    <span aria-live="polite">Page {{ page }} of {{ pages }} · {{ total }} results</span>
    <div class="flex gap-2">
      <button class="ui-button" :disabled="loading || page <= 1" @click="emit('change', page - 1)">Previous</button>
      <button class="ui-button" :disabled="loading || page >= pages" @click="emit('change', page + 1)">Next</button>
    </div>
  </nav>
</template>
