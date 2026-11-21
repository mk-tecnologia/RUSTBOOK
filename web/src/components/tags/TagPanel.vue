<script setup lang="ts">
import type { Tag } from '../../api/addressBook'
import { tagHex } from '../../utils/colors'
defineProps<{ tags: Tag[]; selectedTag: string | null; busy?: boolean }>()
const emit = defineEmits<{ select: [tag: string | null]; create: []; delete: [name: string] }>()
</script>
<template>
  <div class="space-y-1">
    <div class="mb-3 flex items-center justify-between px-1">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-rd-text-secondary">Tags</h2>
      <button type="button" :disabled="busy" aria-label="Create tag" title="Create tag" class="rounded-md p-1.5 text-gray-400 hover:bg-gray-100 hover:text-rd-primary dark:hover:bg-rd-card-hover" @click="emit('create')"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.8" viewBox="0 0 24 24"><path stroke-linecap="round" d="M12 5v14M5 12h14" /></svg></button>
    </div>
    <button type="button" :aria-pressed="!selectedTag" class="flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition-colors" :class="!selectedTag ? 'bg-rd-primary/10 font-medium text-rd-primary' : 'text-gray-600 hover:bg-gray-100 dark:text-rd-text-secondary dark:hover:bg-rd-card-hover'" @click="emit('select', null)"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24"><path d="M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z" /></svg>All tags</button>
    <div v-for="tag in tags" :key="tag.name" class="group flex items-center gap-1">
      <button type="button" :aria-pressed="selectedTag === tag.name" class="flex min-w-0 flex-1 items-center gap-2.5 rounded-lg px-3 py-2 text-left text-sm transition-colors" :class="selectedTag === tag.name ? 'bg-rd-primary/10 font-medium text-rd-primary' : 'text-gray-600 hover:bg-gray-100 dark:text-rd-text-secondary dark:hover:bg-rd-card-hover'" @click="emit('select', tag.name)"><span class="size-2.5 shrink-0 rounded-full" :style="{ backgroundColor: tagHex(tag.color) }" /><span class="truncate">{{ tag.name }}</span></button>
      <button type="button" :disabled="busy" :aria-label="`Delete tag ${tag.name}`" title="Delete tag" class="rounded p-1.5 text-gray-400 hover:text-rd-danger md:opacity-0 md:group-hover:opacity-100 focus:opacity-100" @click="emit('delete', tag.name)"><svg aria-hidden="true" class="size-3.5" fill="none" stroke="currentColor" stroke-width="1.8" viewBox="0 0 24 24"><path stroke-linecap="round" d="m6 6 12 12M6 18 18 6" /></svg></button>
    </div>
    <p v-if="!tags.length" class="px-3 pt-3 text-xs leading-relaxed text-gray-400 dark:text-rd-text-secondary">Create tags to label devices across your groups.</p>
  </div>
</template>
