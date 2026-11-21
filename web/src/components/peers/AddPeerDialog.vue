<script setup lang="ts">
import { ref } from 'vue'
import BaseDialog from '../ui/BaseDialog.vue'
import ErrorNotice from '../ui/ErrorNotice.vue'
const props = defineProps<{ busy?: boolean; serverError?: string }>()

const emit = defineEmits<{
  close: []
  add: [peer: { id: string; alias: string; platform: string; tags: string[] }]
}>()

const id = ref('')
const alias = ref('')
const platform = ref('Windows')
const error = ref('')

const platforms = ['Windows', 'Mac OS', 'Linux', 'Android', 'iOS']

function submit() {
  if (props.busy) return
  if (!id.value.trim()) {
    error.value = 'RustDesk ID is required'
    return
  }
  emit('add', {
    id: id.value.trim(),
    alias: alias.value.trim(),
    platform: platform.value,
    tags: [],
  })
}
</script>

<template>
  <BaseDialog title="Add Peer" :busy="busy" @close="emit('close')">
    <form @submit.prevent="submit">
      <ErrorNotice :message="serverError || ''" />
      <div v-if="error" class="mb-4 p-3 text-sm text-red-600 bg-red-50 dark:bg-red-900/20 dark:text-red-400 rounded-lg">
        {{ error }}
      </div>

      <div class="space-y-4">
        <div>
          <label for="peer-id" class="block text-sm font-medium text-gray-700 dark:text-rd-text-secondary mb-1">RustDesk ID *</label>
          <input
            id="peer-id"
            v-model="id"
            type="text"
            required
            autofocus
            class="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-rd-border bg-white dark:bg-rd-scaffold text-gray-900 dark:text-rd-text focus:outline-none focus:ring-2 focus:ring-rd-primary text-sm"
            placeholder="e.g. 123456789"
          />
        </div>
        <div>
          <label for="peer-alias" class="block text-sm font-medium text-gray-700 dark:text-rd-text-secondary mb-1">Alias</label>
          <input
            id="peer-alias"
            v-model="alias"
            type="text"
            class="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-rd-border bg-white dark:bg-rd-scaffold text-gray-900 dark:text-rd-text focus:outline-none focus:ring-2 focus:ring-rd-primary text-sm"
            placeholder="e.g. Office PC"
          />
        </div>
        <div>
          <label for="peer-platform" class="block text-sm font-medium text-gray-700 dark:text-rd-text-secondary mb-1">Platform</label>
          <select
            id="peer-platform"
            v-model="platform"
            class="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-rd-border bg-white dark:bg-rd-scaffold text-gray-900 dark:text-rd-text focus:outline-none focus:ring-2 focus:ring-rd-primary text-sm"
          >
            <option v-for="p in platforms" :key="p" :value="p">{{ p }}</option>
          </select>
        </div>
      </div>

      <div class="flex justify-end gap-3 mt-6">
        <button
          type="button" :disabled="busy" @click="emit('close')"
          class="px-4 py-2 text-sm rounded-lg text-gray-600 dark:text-rd-text-secondary hover:bg-gray-100 dark:hover:bg-rd-card-hover transition-colors"
        >
          Cancel
        </button>
        <button
          type="submit" :disabled="busy"
          class="px-4 py-2 text-sm rounded-lg bg-rd-primary hover:bg-rd-primary-hover text-white font-medium transition-colors"
        >
          Add Peer
        </button>
      </div>
    </form>
  </BaseDialog>
</template>
