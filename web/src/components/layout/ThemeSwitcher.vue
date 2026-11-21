<script setup lang="ts">
import { useId } from 'vue'
import { useUiStore, type ThemeMode } from '../../stores/ui'

const ui = useUiStore()
const groupName = `theme-${useId()}`
const options: { value: ThemeMode; label: string; path: string }[] = [
  {
    value: 'light',
    label: 'Light theme',
    path: 'M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364-.707.707M6.343 6.343l-.707-.707m12.728 0-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z',
  },
  {
    value: 'dark',
    label: 'Dark theme',
    path: 'M20.354 15.354A9 9 0 0 1 8.646 3.646 9.003 9.003 0 0 0 12 21a9.003 9.003 0 0 0 8.354-5.646Z',
  },
  {
    value: 'system',
    label: 'System theme',
    path: 'M5 4h14a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Zm3 17h8m-4-4v4',
  },
]
</script>

<template>
  <fieldset class="inline-flex shrink-0 items-center gap-0.5 rounded-full border border-gray-200/80 bg-gray-100/80 p-0.5 dark:border-rd-border/70 dark:bg-rd-scaffold/70">
    <legend class="sr-only">Appearance</legend>
    <label v-for="option in options" :key="option.value" :title="option.label" class="relative cursor-pointer">
      <input
        type="radio"
        :name="groupName"
        :value="option.value"
        :aria-label="option.label"
        :checked="ui.theme === option.value"
        class="peer sr-only"
        @change="ui.setTheme(option.value)"
      >
      <span
        class="flex size-9 items-center justify-center rounded-full transition-colors duration-150 peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-rd-primary motion-reduce:transition-none"
        :class="ui.theme === option.value
          ? 'bg-white text-rd-primary shadow-sm ring-1 ring-black/5 dark:bg-rd-card-hover dark:text-rd-text dark:ring-white/10'
          : 'text-gray-500 hover:bg-white/60 hover:text-gray-900 dark:text-rd-text-secondary dark:hover:bg-rd-card/80 dark:hover:text-rd-text'"
      >
        <svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24">
          <path :d="option.path" />
        </svg>
      </span>
    </label>
  </fieldset>
</template>
