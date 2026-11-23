<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../../api/client'
import type { NativeConnection } from '../../utils/connection'
import BaseDialog from '../ui/BaseDialog.vue'
import ErrorNotice from '../ui/ErrorNotice.vue'
interface Settings extends NativeConnection { can_edit: boolean }
const emit = defineEmits<{ changed: [profile: NativeConnection] }>()
const settings = ref<Settings>()
const opened = ref(false)
const busy = ref(false)
const error = ref('')
const server = ref('')
const publicKey = ref('')
async function load() {
  try { settings.value = await api<Settings>('/api/connection-settings'); emit('changed', settings.value) }
  catch { error.value = 'Unable to load native connection settings.' }
}
function open() { server.value = settings.value?.server || ''; publicKey.value = settings.value?.public_key || ''; error.value = ''; opened.value = true }
async function save() {
  if (busy.value) return
  busy.value = true; error.value = ''
  try {
    settings.value = await api<Settings>('/api/admin/connection-settings', { method: 'PUT', body: JSON.stringify({ server: server.value.trim(), public_key: publicKey.value.trim() }) })
    emit('changed', settings.value); opened.value = false
  } catch (e) { error.value = e instanceof Error ? e.message : 'Unable to save connection settings' }
  finally { busy.value = false }
}
onMounted(load)
</script>
<template>
  <div v-if="settings?.can_edit || (error && !settings)" class="min-w-0">
  <div v-if="settings?.can_edit" class="flex h-full min-w-0 flex-col items-start justify-between gap-3 rounded-xl border border-gray-200 bg-white p-4 dark:border-rd-border dark:bg-rd-card">
    <div class="w-full min-w-0"><p class="text-sm font-medium">Native connections</p><p class="mt-1 line-clamp-2 text-xs text-gray-500 dark:text-rd-text-secondary">{{ settings.server ? `ID server: ${settings.server} · public key configured` : 'Configure the ID server and public Key for hbbs OSS connections while signed in.' }}</p></div><button type="button" class="ui-button !px-3 !py-1.5 !text-xs" @click="open">Connection settings</button>
  </div>
  <ErrorNotice v-if="error && !settings" :message="error" retry @retry="load" />
  </div>
  <BaseDialog v-if="opened" title="Native connections" :busy="busy" @close="opened = false">
    <form class="space-y-4" @submit.prevent="save">
      <ErrorNotice :message="error" />
      <div><label for="connection-server" class="mb-1 block text-xs font-medium">ID server · TCP endpoint</label><input id="connection-server" v-model="server" :disabled="busy" maxlength="255" autocomplete="off" placeholder="rustdesk.example.com:21116" class="peer-input"><p class="mt-1 text-xs text-gray-500 dark:text-rd-text-secondary">Use an address reachable from the computers running RustDesk. This is the ID port, usually 21116, not the presence port.</p></div>
      <div><label for="connection-key" class="mb-1 block text-xs font-medium">Public Key</label><input id="connection-key" v-model="publicKey" :disabled="busy" maxlength="44" autocomplete="off" spellcheck="false" class="peer-input font-mono" placeholder="Public Key from RustDesk server settings"><p class="mt-1 text-xs text-gray-500 dark:text-rd-text-secondary">Copy the public Key already configured in your clients. The server public key is included in the connection link.</p></div>
      <p class="text-xs text-gray-500 dark:text-rd-text-secondary">Explicit server connections support hbbs OSS while the client is signed in to the address book. Remote password/approval is still required. Clear both fields to use the client's own default connection settings.</p>
      <div class="flex justify-end gap-2 border-t border-gray-100 pt-4 dark:border-rd-border"><button type="button" :disabled="busy" class="ui-button" @click="opened = false">Cancel</button><button type="submit" :disabled="busy" class="rounded-lg bg-rd-primary px-4 py-2 text-sm font-medium text-white hover:bg-rd-primary-hover disabled:opacity-50">{{ busy ? 'Saving…' : 'Save changes' }}</button></div>
    </form>
  </BaseDialog>
</template>
