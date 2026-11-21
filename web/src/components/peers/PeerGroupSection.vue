<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { getPeers, type Peer, type PeerGroup, type Tag } from '../../api/addressBook'
import { tagHex } from '../../utils/colors'
import ErrorNotice from '../ui/ErrorNotice.vue'
import PaginationControls from '../ui/PaginationControls.vue'
import PresenceBadge from './PresenceBadge.vue'
import { presenceSummary } from '../../utils/presence'
import { rustdeskLink, type NativeConnection } from '../../utils/connection'

const props = defineProps<{
  connection?: NativeConnection; guid: string; group: PeerGroup | null; count: number; tags: Tag[]
  query: string; selectedTag: string | null; refreshKey: number; initialOpen?: boolean; busy: boolean
}>()
const emit = defineEmits<{ add: [groupId: number | null]; edit: [peer: Peer]; remove: [peer: Peer]; editGroup: [group: PeerGroup]; removeGroup: [group: PeerGroup] }>()
const expanded = ref(Boolean(props.initialOpen))
const peers = ref<Peer[]>([])
const page = ref(1)
const total = ref(props.count)
const pageSize = 24
const loading = ref(false)
const error = ref('')
const loaded = ref(false)
const refreshError = ref('')
const tagColors = computed(() => new Map(props.tags.map(tag => [tag.name, tagHex(tag.color)])))
let active: AbortController | undefined
let refreshTimer: ReturnType<typeof setInterval> | undefined
async function load(background = false) {
  active?.abort()
  const request = new AbortController(); active = request
  if (!background) loading.value = true
  error.value = ''
  try {
    const result = await getPeers(props.guid, page.value, pageSize, props.query, props.selectedTag || '', request.signal, props.group ? String(props.group.id) : 'ungrouped')
    if (request.signal.aborted) return
    peers.value = result.data; total.value = result.total; loaded.value = true
    refreshError.value = ''
    const last = Math.max(1, Math.ceil(result.total / pageSize))
    if (page.value > last) { page.value = last; await load() }
  } catch (e) {
    if (!request.signal.aborted) {
      if (background) {
        refreshError.value = 'Unable to refresh presence. Displaying devices without a confirmed status.'
        peers.value = peers.value.map(peer => ({ ...peer, online: undefined, presence_source: 'probe_failed' }))
      } else error.value = e instanceof Error ? e.message : 'Unable to load devices'
    }
  } finally { if (active === request) { loading.value = false; active = undefined } }
}
function toggle(event: Event) {
  expanded.value = (event.currentTarget as HTMLDetailsElement).open
  if (expanded.value && !active) void load(loaded.value)
}
function refreshPresence() {
  if (expanded.value && loaded.value && !active && !props.busy && document.visibilityState !== 'hidden') void load(true)
}
async function setPage(next: number) { page.value = next; await load() }
watch(() => [props.guid, props.query, props.selectedTag, props.refreshKey], () => {
  active?.abort(); active = undefined; loading.value = false
  loaded.value = false; page.value = 1; peers.value = []; error.value = ''
  if (expanded.value) void load()
})
onMounted(() => {
  if (expanded.value && !active) void load()
  refreshTimer = setInterval(refreshPresence, 30_000)
  window.addEventListener('focus', refreshPresence)
})
onBeforeUnmount(() => { active?.abort(); clearInterval(refreshTimer); window.removeEventListener('focus', refreshPresence) })
</script>

<template>
  <details :open="expanded" class="group/section" @toggle="toggle">
    <summary class="flex cursor-pointer list-none items-center gap-3 bg-gray-50/90 px-4 py-3 transition-colors hover:bg-gray-100/80 dark:bg-rd-scaffold/45 dark:hover:bg-rd-card-hover [&::-webkit-details-marker]:hidden">
      <svg aria-hidden="true" class="size-3.5 shrink-0 text-gray-400 transition-transform group-open/section:rotate-90 motion-reduce:transition-none" fill="none" stroke="currentColor" stroke-width="1.8" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="m9 5 7 7-7 7" /></svg>
      <svg aria-hidden="true" class="size-5 shrink-0" :class="group ? 'text-amber-500' : 'text-gray-400'" fill="none" stroke="currentColor" stroke-width="1.6" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M3 7v11a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-6l-2-3H5a2 2 0 0 0-2 2v1Z" /></svg>
      <span class="min-w-0 flex-1"><span class="block truncate text-sm font-medium text-gray-800 dark:text-rd-text">{{ group?.name || 'Ungrouped' }}</span><span v-if="group?.note" class="mt-0.5 block truncate text-xs text-gray-400 dark:text-rd-text-secondary">{{ group.note }}</span></span>
      <span class="rounded-md bg-white px-2 py-0.5 text-xs tabular-nums text-gray-500 ring-1 ring-gray-200 dark:bg-rd-card dark:text-rd-text-secondary dark:ring-rd-border">{{ count }}</span>
      <span class="flex shrink-0 items-center gap-0.5">
        <button type="button" :disabled="busy" :aria-label="`Add device to ${group?.name || 'Ungrouped'}`" title="Add device" class="rounded-md p-1.5 text-gray-400 hover:bg-white hover:text-rd-primary dark:hover:bg-rd-card" @click.prevent.stop="emit('add', group?.id ?? null)"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.8" viewBox="0 0 24 24"><path stroke-linecap="round" d="M12 5v14M5 12h14" /></svg></button>
        <template v-if="group">
          <button type="button" :disabled="busy" :aria-label="`Edit group ${group.name}`" title="Edit group" class="rounded-md p-1.5 text-gray-400 hover:bg-white hover:text-rd-primary dark:hover:bg-rd-card" @click.prevent.stop="emit('editGroup', group)"><svg aria-hidden="true" class="size-3.5" fill="none" stroke="currentColor" stroke-width="1.8" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="m16 4 4 4M4 20l4-1L20 7a2.8 2.8 0 0 0-4-4L4 15l-1 6Z" /></svg></button>
          <button type="button" :disabled="busy" :aria-label="`Delete group ${group.name}`" title="Delete group; keep devices" class="rounded-md p-1.5 text-gray-400 hover:bg-white hover:text-rd-danger dark:hover:bg-rd-card" @click.prevent.stop="emit('removeGroup', group)"><svg aria-hidden="true" class="size-3.5" fill="none" stroke="currentColor" stroke-width="1.8" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M4 7h16M9 7V4h6v3m-9 0 1 13h10l1-13M10 11v5m4-5v5" /></svg></button>
        </template>
      </span>
    </summary>
    <div v-if="expanded" class="bg-white dark:bg-rd-card">
      <p v-if="refreshError" role="status" class="px-4 py-2 text-xs text-amber-700 dark:text-amber-400">{{ refreshError }}</p>
      <div v-if="loading" role="status" class="px-6 py-5 text-sm text-gray-400">Loading devices…</div>
      <div v-else-if="error" class="p-4"><ErrorNotice :message="error" retry @retry="() => load()" /></div>
      <div v-else-if="!peers.length" class="flex flex-wrap items-center justify-between gap-3 px-6 py-6 text-sm text-gray-400 dark:text-rd-text-secondary"><span>{{ query || selectedTag ? 'No devices match these filters.' : 'No devices in this group yet.' }}</span><button v-if="!query && !selectedTag" type="button" :disabled="busy" class="font-medium text-rd-primary" @click="emit('add', group?.id ?? null)">Add a device</button></div>
      <div v-else class="overflow-x-auto">
        <table class="w-full min-w-[700px] text-left text-sm">
          <caption class="sr-only">Devices in {{ group?.name || 'Ungrouped' }}</caption>
          <thead><tr class="border-b border-gray-100 text-[10px] font-medium uppercase tracking-wider text-gray-400 dark:border-rd-border/60 dark:text-rd-text-secondary"><th class="py-2 pl-12 pr-4 font-medium">Name</th><th class="px-4 py-2 font-medium">RustDesk ID</th><th class="px-4 py-2 font-medium">Tags</th><th class="px-4 py-2 font-medium">Status</th><th class="px-4 py-2 text-right font-medium"><span class="sr-only">Actions</span></th></tr></thead>
          <tbody>
            <tr v-for="peer in peers" :key="peer.id" class="group/peer border-b border-gray-100 last:border-b-0 hover:bg-gray-50/80 dark:border-rd-border/40 dark:hover:bg-rd-card-hover/70">
              <td class="py-3 pl-12 pr-4"><div class="flex min-w-0 items-center gap-3"><span class="relative flex size-8 shrink-0 items-center justify-center rounded-lg bg-gray-100 text-gray-500 dark:bg-rd-scaffold dark:text-rd-text-secondary"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M4 4h16v12H4zM8 20h8m-4-4v4" /></svg><span v-if="presenceSummary(peer).tone !== 'unknown'" class="absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full border-2 border-white dark:border-rd-card" :class="presenceSummary(peer).tone === 'online' ? 'bg-rd-success' : presenceSummary(peer).tone === 'recent' ? 'bg-rd-primary' : 'bg-gray-400'" /></span><div><button type="button" class="text-left font-medium text-gray-800 hover:text-rd-primary dark:text-rd-text" @click="emit('edit', peer)">{{ peer.alias || peer.hostname || peer.id }}</button><p class="mt-0.5 text-xs text-gray-400 dark:text-rd-text-secondary">{{ peer.platform || 'Unknown platform' }}<span v-if="peer.username"> · {{ peer.username }}</span></p></div></div></td>
              <td class="whitespace-nowrap px-4 py-3 font-mono text-xs text-gray-500 dark:text-rd-text-secondary">{{ peer.id }}</td>
              <td class="px-4 py-3"><div class="flex max-w-64 flex-wrap gap-1"><span v-for="tag in peer.tags" :key="tag" class="inline-flex items-center gap-1.5 rounded-md border border-gray-200/80 bg-gray-50 px-2 py-0.5 text-[11px] text-gray-600 dark:border-rd-border dark:bg-rd-scaffold/60 dark:text-rd-text-secondary"><span class="size-1.5 shrink-0 rounded-full" :style="{ backgroundColor: tagColors.get(tag) || '#0071ff' }" />{{ tag }}</span><span v-if="!peer.tags.length" class="text-xs text-gray-300 dark:text-rd-text-secondary">—</span></div></td>
              <td class="px-4 py-3"><PresenceBadge :peer="peer" /></td>
              <td class="px-4 py-3"><div class="flex justify-end gap-1"><a v-if="connection && rustdeskLink(peer.id, connection)" :href="rustdeskLink(peer.id, connection)!" :aria-label="`Connect to ${peer.alias || peer.id} in RustDesk`" title="Connect in RustDesk" class="inline-flex size-8 items-center justify-center rounded-md text-blue-600 transition-colors hover:bg-blue-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 dark:text-blue-400 dark:hover:bg-blue-500/10"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.7" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M10 4H3v13h18v-6M7 21h10m-5-4v4M14 3h7v7m0-7-9 9" /></svg></a><button type="button" :disabled="busy" :aria-label="`Edit device ${peer.alias || peer.id}`" title="Edit device, group and tags" class="rounded-md p-2 text-gray-400 hover:bg-gray-100 hover:text-rd-primary dark:hover:bg-rd-scaffold" @click="emit('edit', peer)"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.7" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="m16 4 4 4M4 20l4-1L20 7a2.8 2.8 0 0 0-4-4L4 15l-1 6Z" /></svg></button><button type="button" :disabled="busy" :aria-label="`Remove device ${peer.alias || peer.id}`" title="Remove device" class="rounded-md p-2 text-gray-400 hover:bg-red-50 hover:text-rd-danger dark:hover:bg-red-950/20" @click="emit('remove', peer)"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.7" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M4 7h16M9 7V4h6v3m-9 0 1 13h10l1-13M10 11v5m4-5v5" /></svg></button></div></td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-if="!error && total > pageSize" class="border-t border-gray-100 px-4 pb-4 dark:border-rd-border"><PaginationControls :page="page" :page-size="pageSize" :total="total" :loading="loading" @change="setPage" /></div>
    </div>
  </details>
</template>
