<script setup lang="ts">
import type { Tag } from '../../api/addressBook'
import TagSelect from './TagSelect.vue'
defineProps<{ tags: Tag[]; selectedTag: string | null; busy?: boolean }>()
const emit = defineEmits<{ select: [tag: string | null]; create: []; delete: [name: string] }>()
</script>
<template>
  <div class="flex h-full min-w-0 flex-col rounded-xl border border-gray-200 bg-white p-4 dark:border-rd-border dark:bg-rd-card">
    <div class="mb-3 flex items-center justify-between gap-2"><div><h2 class="text-sm font-medium">Tags</h2><p class="mt-0.5 text-xs text-gray-500 dark:text-rd-text-secondary">Filter devices by label</p></div><div class="flex items-center gap-1">
      <button v-if="selectedTag" type="button" :disabled="busy" :aria-label="`Delete tag ${selectedTag}`" title="Delete selected tag" class="rounded-lg p-2 text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10" @click="emit('delete', selectedTag)"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.6" viewBox="0 0 24 24"><path d="M4 7h16M9 7V4h6v3m-9 0 1 13h10l1-13M10 11v5m4-5v5" /></svg></button>
      <button type="button" :disabled="busy" aria-label="Create tag" title="Create tag" class="rounded-lg p-2 text-rd-primary hover:bg-rd-primary/10" @click="emit('create')"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.8" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg></button>
    </div></div>
    <TagSelect id="device-tags-filter" :tags="tags" :model-value="selectedTag ? [selectedTag] : []" :disabled="busy" label="Filter tags" placeholder="All tags" @update:model-value="emit('select', $event[0] || null)" />
  </div>
</template>
