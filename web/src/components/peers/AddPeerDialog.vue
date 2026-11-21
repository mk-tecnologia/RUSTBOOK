<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Peer, PeerForm, PeerGroup, Tag } from '../../api/addressBook'
import { tagColor, tagHex } from '../../utils/colors'
import BaseDialog from '../ui/BaseDialog.vue'
import ErrorNotice from '../ui/ErrorNotice.vue'

const props = defineProps<{
  busy?: boolean; serverError?: string; peer?: Peer; groups: PeerGroup[]; tags: Tag[]
  initialGroup?: number | null
  initialTags?: string[]
  createTag: (name: string, color: number) => Promise<void>
}>()
const emit = defineEmits<{ close: []; save: [peer: PeerForm] }>()
const form = ref<PeerForm>({
  id: props.peer?.id || '', alias: props.peer?.alias || '', platform: props.peer?.platform || 'Windows',
  hostname: props.peer?.hostname || '', username: props.peer?.username || '', note: props.peer?.note || '',
  tags: [...(props.peer?.tags || props.initialTags || [])], group_id: props.peer?.group_id ?? props.initialGroup ?? null,
})
const error = ref('')
const tagName = ref('')
const color = ref('#0071ff')
const creatingTag = ref(false)
const moreDetails = ref(Boolean(form.value.hostname || form.value.username || form.value.note))
const platforms = computed(() => [...new Set(['Windows', 'Mac OS', 'macOS', 'Linux', 'Android', 'iOS', form.value.platform])])
async function addTag() {
  const name = tagName.value.trim()
  if (!name || creatingTag.value || props.busy) return
  error.value = ''; creatingTag.value = true
  try {
    await props.createTag(name, tagColor(color.value))
    if (!form.value.tags.includes(name)) form.value.tags.push(name)
    tagName.value = ''
  } catch (e) { error.value = e instanceof Error ? e.message : 'Unable to create tag' }
  finally { creatingTag.value = false }
}
function submit() {
  if (props.busy || creatingTag.value) return
  error.value = ''
  if (!form.value.id.trim()) { error.value = 'RustDesk ID is required'; return }
  emit('save', { ...form.value, id: form.value.id.trim(), alias: form.value.alias.trim(), tags: [...form.value.tags] })
}
</script>

<template>
  <BaseDialog :title="peer ? 'Edit device' : 'Add device'" :busy="busy || creatingTag" @close="emit('close')">
    <form class="space-y-4" @submit.prevent="submit">
      <ErrorNotice :message="serverError || error" />
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div>
          <label for="peer-id" class="mb-1 block text-xs font-medium">RustDesk ID *</label>
          <input id="peer-id" v-model="form.id" required maxlength="128" :readonly="Boolean(peer)" autofocus class="peer-input read-only:bg-gray-100 dark:read-only:bg-rd-scaffold" placeholder="123456789">
        </div>
        <div>
          <label for="peer-alias" class="mb-1 block text-xs font-medium">Device name</label>
          <input id="peer-alias" v-model="form.alias" class="peer-input" placeholder="Office PC">
        </div>
        <div>
          <label for="peer-platform" class="mb-1 block text-xs font-medium">Platform</label>
          <select id="peer-platform" v-model="form.platform" class="peer-input"><option v-for="item in platforms" :key="item" :value="item">{{ item }}</option></select>
        </div>
        <div>
          <label for="peer-group" class="mb-1 block text-xs font-medium">Device group</label>
          <select id="peer-group" v-model="form.group_id" class="peer-input">
            <option :value="null">Ungrouped</option>
            <option v-for="group in groups" :key="group.id" :value="group.id">{{ group.name }}</option>
          </select>
        </div>
      </div>
      <fieldset class="rounded-xl border border-gray-200 p-3 dark:border-rd-border">
        <legend class="px-1 text-xs font-medium">Tags <span class="font-normal text-gray-400">· choose multiple</span></legend>
        <div v-if="tags.length" class="mb-3 flex max-h-32 flex-wrap gap-2 overflow-y-auto p-0.5">
          <label v-for="tag in tags" :key="tag.name" class="flex cursor-pointer items-center gap-2 rounded-lg border px-2.5 py-1.5 text-xs transition-colors" :class="form.tags.includes(tag.name) ? 'border-rd-primary bg-rd-primary/5 dark:bg-rd-primary/10' : 'border-gray-200 dark:border-rd-border'">
            <input v-model="form.tags" type="checkbox" :value="tag.name" class="accent-rd-primary">
            <span class="size-2 shrink-0 rounded-full" :style="{ backgroundColor: tagHex(tag.color) }" />{{ tag.name }}
          </label>
        </div>
        <p v-else class="mb-3 text-xs text-gray-500">Create your first tag below to label this device.</p>
        <div class="flex items-center gap-2">
          <input v-model="color" type="color" aria-label="New tag color" class="size-9 shrink-0 cursor-pointer rounded border border-gray-200 bg-transparent p-1 dark:border-rd-border">
          <input v-model="tagName" aria-label="New tag name" maxlength="128" class="peer-input min-w-0 flex-1" placeholder="New tag…" @keydown.enter.prevent="addTag">
          <button type="button" :disabled="!tagName.trim() || creatingTag || busy" class="ui-button shrink-0 !px-3" @click="addTag">{{ creatingTag ? 'Adding…' : 'Add' }}</button>
        </div>
      </fieldset>
      <button type="button" class="text-xs font-medium text-rd-primary" :aria-expanded="moreDetails" @click="moreDetails = !moreDetails">{{ moreDetails ? 'Hide details' : 'More details' }}</button>
      <div v-if="moreDetails" class="space-y-3">
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div><label for="peer-hostname" class="mb-1 block text-xs font-medium">Hostname</label><input id="peer-hostname" v-model="form.hostname" class="peer-input"></div>
          <div><label for="peer-username" class="mb-1 block text-xs font-medium">Username</label><input id="peer-username" v-model="form.username" class="peer-input"></div>
        </div>
        <div><label for="peer-note" class="mb-1 block text-xs font-medium">Notes</label><textarea id="peer-note" v-model="form.note" rows="2" class="peer-input" /></div>
      </div>
      <div class="flex justify-end gap-2 border-t border-gray-100 pt-4 dark:border-rd-border">
        <button type="button" :disabled="busy || creatingTag" class="ui-button" @click="emit('close')">Cancel</button>
        <button type="submit" :disabled="busy || creatingTag" class="rounded-lg bg-rd-primary px-4 py-2 text-sm font-medium text-white hover:bg-rd-primary-hover disabled:opacity-50">{{ busy ? 'Saving…' : peer ? 'Save changes' : 'Add device' }}</button>
      </div>
    </form>
  </BaseDialog>
</template>
