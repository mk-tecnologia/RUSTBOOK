<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api, ApiError } from '../../api/client'
import BaseDialog from '../ui/BaseDialog.vue'
import ErrorNotice from '../ui/ErrorNotice.vue'
interface Settings { server: string; source: 'saved' | 'deployment'; status: 'connected' | 'not_configured' | 'unreachable' }
const emit = defineEmits<{ changed: [] }>()
const settings = ref<Settings>()
const opened = ref(false)
const server = ref('')
const busy = ref(false)
const error = ref('')
const loadError = ref('')
async function load() {
  loadError.value = ''
  try { settings.value = await api<Settings>('/api/admin/presence') }
  catch (e) { if (!(e instanceof ApiError && e.status === 403)) loadError.value = 'Unable to check presence configuration.' }
}
function open() { server.value = settings.value?.server || ''; error.value = ''; opened.value = true }
async function save(reset = false) {
  if (busy.value) return
  busy.value = true; error.value = ''
  try {
    settings.value = await api<Settings>('/api/admin/presence', reset ? { method: 'DELETE' } : { method: 'PUT', body: JSON.stringify({ server: server.value.trim() }) })
    opened.value = false; emit('changed')
  } catch (e) { error.value = e instanceof Error ? e.message : 'Unable to save presence settings' }
  finally { busy.value = false }
}
onMounted(load)
</script>
<template>
  <div v-if="settings || loadError">
    <div v-if="settings" class="flex flex-wrap items-center justify-between gap-3 rounded-xl border px-4 py-3 text-sm" :class="settings.status === 'connected' ? 'border-gray-200 bg-white dark:border-rd-border dark:bg-rd-card' : 'border-amber-200 bg-amber-50 dark:border-amber-900/60 dark:bg-amber-950/20'">
      <div><p class="font-medium text-gray-800 dark:text-rd-text">{{ settings.status === 'connected' ? 'Live presence connected' : settings.status === 'not_configured' ? 'Configure live presence' : 'ID server unavailable' }}</p><p class="mt-0.5 text-xs text-gray-500 dark:text-rd-text-secondary">{{ settings.status === 'connected' ? `ID server: ${settings.server}` : settings.status === 'not_configured' ? 'Select your RustDesk ID server to show confirmed Online/Offline status.' : `Unable to query ${settings.server}. Device presence remains unknown.` }}</p></div>
      <button type="button" class="ui-button" @click="open">Presence settings</button>
    </div>
    <ErrorNotice v-else :message="loadError" retry @retry="load" />
    <BaseDialog v-if="opened" title="Live presence" :busy="busy" @close="opened = false">
      <form class="space-y-4" @submit.prevent="save()">
        <ErrorNotice :message="error" />
        <div><label for="presence-server" class="mb-1 block text-xs font-medium">RustDesk ID server · NAT TCP endpoint</label><input id="presence-server" v-model="server" :disabled="busy" maxlength="255" autocomplete="off" placeholder="rustdesk.example.com:21115" class="peer-input"><p class="mt-2 text-xs leading-relaxed text-gray-500 dark:text-rd-text-secondary">Use the same ID server as your clients, with its NAT TCP port (usually 21115). Enter a domain, IP or Docker service name reachable from RUSTBOOK. Leave empty to disable live queries.</p></div>
        <p class="text-xs text-gray-500 dark:text-rd-text-secondary">The connection is tested before saving. This setting is stored for this installation and takes priority over the deployment default.</p>
        <div class="flex flex-wrap justify-end gap-2 border-t border-gray-100 pt-4 dark:border-rd-border"><button v-if="settings?.source === 'saved'" type="button" :disabled="busy" class="ui-button mr-auto" @click="save(true)">Use deployment default</button><button type="button" :disabled="busy" class="ui-button" @click="opened = false">Cancel</button><button type="submit" :disabled="busy" class="rounded-lg bg-rd-primary px-4 py-2 text-sm font-medium text-white hover:bg-rd-primary-hover disabled:opacity-50">{{ busy ? 'Testing…' : 'Test and save' }}</button></div>
      </form>
    </BaseDialog>
  </div>
</template>
