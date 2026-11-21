<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from 'vue'
defineProps<{ title: string; busy?: boolean }>()
const emit = defineEmits<{ close: [] }>()
const dialog = ref<HTMLDialogElement>()
let previous: HTMLElement | null = null
onMounted(() => {
  previous = document.activeElement as HTMLElement | null
  dialog.value?.showModal()
})
onBeforeUnmount(() => { dialog.value?.close(); previous?.focus() })
</script>
<template>
  <dialog ref="dialog" aria-labelledby="dialog-title" class="w-[calc(100%_-_2rem)] max-w-lg max-h-[calc(100dvh_-_2rem)] overflow-y-auto rounded-2xl p-6 border border-gray-200 bg-white text-gray-900 shadow-xl dark:bg-rd-card dark:border-rd-border dark:text-rd-text backdrop:bg-black/50" @cancel.prevent="!busy && emit('close')">
    <h2 id="dialog-title" class="text-lg font-semibold mb-4">{{ title }}</h2>
    <slot />
  </dialog>
</template>
