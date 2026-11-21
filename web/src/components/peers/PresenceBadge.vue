<script setup lang="ts">
import { computed } from 'vue'
import type { Peer } from '../../api/addressBook'
import { presenceSummary } from '../../utils/presence'
const props = defineProps<{ peer: Peer }>()
const presence = computed(() => presenceSummary(props.peer))
</script>
<template>
  <span :title="presence.description" :aria-label="`${presence.label}. ${presence.description}`" class="inline-flex items-center gap-1.5 whitespace-nowrap rounded-full px-2 py-0.5 text-[11px]" :class="presence.tone === 'online' ? 'bg-green-50 text-green-700 dark:bg-green-950/40 dark:text-green-400' : presence.tone === 'recent' ? 'bg-blue-50 text-blue-700 dark:bg-blue-950/40 dark:text-blue-300' : 'bg-gray-100 text-gray-500 dark:bg-rd-scaffold dark:text-rd-text-secondary'">
    <span aria-hidden="true" class="size-1.5 rounded-full" :class="presence.tone === 'online' ? 'bg-rd-success' : presence.tone === 'recent' ? 'bg-rd-primary' : 'bg-gray-400'" />{{ presence.label }}
  </span>
</template>
