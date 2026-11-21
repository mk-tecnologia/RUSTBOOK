<script setup lang="ts">
import { ref } from 'vue'
import type { PeerGroup } from '../../api/addressBook'
import { tagColor } from '../../utils/colors'
import BaseDialog from '../ui/BaseDialog.vue'
import ErrorNotice from '../ui/ErrorNotice.vue'
const props = defineProps<{ kind: 'group' | 'tag'; group?: PeerGroup; busy: boolean; error: string }>()
const emit = defineEmits<{ close: []; save: [value: { name: string; note: string; color: number }] }>()
const name = ref(props.group?.name || '')
const note = ref(props.group?.note || '')
const color = ref('#0071ff')
function submit() {
  if (!props.busy && name.value.trim()) emit('save', { name: name.value.trim(), note: note.value.trim(), color: tagColor(color.value) })
}
</script>
<template>
  <BaseDialog :title="kind === 'tag' ? 'New tag' : group ? 'Edit device group' : 'New device group'" :busy="busy" @close="emit('close')">
    <form class="space-y-4" @submit.prevent="submit">
      <ErrorNotice :message="error" />
      <div><label for="organization-name" class="mb-1 block text-xs font-medium">Name *</label><input id="organization-name" v-model="name" required maxlength="128" autofocus class="peer-input" :placeholder="kind === 'tag' ? 'Production, Windows, Finance…' : 'Customer or site name'"></div>
      <div v-if="kind === 'group'"><label for="organization-note" class="mb-1 block text-xs font-medium">Description</label><textarea id="organization-note" v-model="note" maxlength="2048" rows="3" class="peer-input" placeholder="Optional description for this group" /></div>
      <div v-else class="flex items-center gap-3"><label for="organization-color" class="text-sm">Tag color</label><input id="organization-color" v-model="color" type="color" class="size-10 cursor-pointer rounded-lg border border-gray-200 bg-transparent p-1 dark:border-rd-border"></div>
      <p class="text-xs leading-relaxed text-gray-500 dark:text-rd-text-secondary">{{ kind === 'group' ? 'Move devices into this group when adding or editing them. Devices can have multiple tags.' : 'Apply this tag when adding or editing a device. A tag can label devices in different groups.' }}</p>
      <div class="flex justify-end gap-2 border-t border-gray-100 pt-4 dark:border-rd-border"><button type="button" :disabled="busy" class="ui-button" @click="emit('close')">Cancel</button><button type="submit" :disabled="busy || !name.trim()" class="rounded-lg bg-rd-primary px-4 py-2 text-sm font-medium text-white hover:bg-rd-primary-hover disabled:opacity-50">{{ busy ? 'Saving…' : group ? 'Save changes' : 'Create' }}</button></div>
    </form>
  </BaseDialog>
</template>
