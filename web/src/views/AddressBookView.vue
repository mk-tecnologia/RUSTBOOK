<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { Peer, PeerForm, PeerGroup } from '../api/addressBook'
import { useAddressBookStore } from '../stores/addressBook'
import PeerGroupSection from '../components/peers/PeerGroupSection.vue'
import TagPanel from '../components/tags/TagPanel.vue'
import AddPeerDialog from '../components/peers/AddPeerDialog.vue'
import OrganizationDialog from '../components/peers/OrganizationDialog.vue'
import ServerOverview from '../components/server/ServerOverview.vue'
import type { NativeConnection } from '../utils/connection'
import ErrorNotice from '../components/ui/ErrorNotice.vue'

const store = useAddressBookStore()
const search = ref(store.query)
const connection = ref<NativeConnection>()
const saving = ref(false)
const actionError = ref('')
const peerEditor = ref<{ peer?: Peer; groupId: number | null } | null>(null)
const organization = ref<{ kind: 'group' | 'tag'; group?: PeerGroup } | null>(null)
const filtered = computed(() => Boolean(store.query || store.selectedTag))
const visibleGroups = computed(() => store.groups.filter(group => !filtered.value || group.peer_count > 0))
const firstGroupId = computed(() => visibleGroups.value.find(group => group.peer_count > 0)?.id ?? visibleGroups.value[0]?.id)
let timer: ReturnType<typeof setTimeout> | undefined
onMounted(store.init)
onBeforeUnmount(() => clearTimeout(timer))
watch(search, () => {
  clearTimeout(timer)
  timer = setTimeout(() => store.setFilters(search.value, store.selectedTag), 250)
}, { flush: 'sync' })
async function selectTag(tag: string | null) {
  clearTimeout(timer)
  await store.setFilters(search.value, tag)
}
async function action(work: () => Promise<void>) {
  if (saving.value) return
  actionError.value = ''; saving.value = true
  try { await work() } catch (e) { actionError.value = e instanceof Error ? e.message : 'Operation failed' }
  finally { saving.value = false }
}
function addPeer(groupId: number | null = null) { actionError.value = ''; peerEditor.value = { groupId } }
function editPeer(peer: Peer) { actionError.value = ''; peerEditor.value = { peer, groupId: peer.group_id ?? null } }
function editOrganization(kind: 'group' | 'tag', group?: PeerGroup) { actionError.value = ''; organization.value = { kind, group } }
async function savePeer(peer: PeerForm) {
  await action(async () => {
    if (peerEditor.value?.peer) await store.updatePeer(peer)
    else await store.addPeer(peer)
    peerEditor.value = null
  })
}
async function saveOrganization(value: { name: string; note: string; color: number }) {
  await action(async () => {
    if (organization.value?.kind === 'tag') await store.addTag(value.name, value.color)
    else {
      const creating = !organization.value?.group
      await store.saveGroup({ name: value.name, note: value.note, id: organization.value?.group?.id })
      if (creating && filtered.value) await clearFilters()
    }
    organization.value = null
  })
}
async function removePeer(peer: Peer) {
  if (confirm(`Remove device "${peer.alias || peer.id}" from this address book?`)) await action(() => store.removePeer([peer.id]))
}
async function removeGroup(group: PeerGroup) {
  if (confirm(`Delete group "${group.name}"? Its devices and tags will be kept in Ungrouped.`)) await action(() => store.removeGroup(group.id))
}
async function removeTag(name: string) {
  if (confirm(`Delete tag "${name}" from all devices? The devices and their groups will be kept.`)) await action(() => store.removeTag([name]))
}
async function clearFilters() { search.value = ''; clearTimeout(timer); await store.setFilters('', null) }
</script>

<template>
  <div class="space-y-6">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div><h1 class="text-xl font-semibold tracking-tight text-gray-900 dark:text-rd-text">Devices</h1><p class="mt-1 text-sm text-gray-500 dark:text-rd-text-secondary">Organize your devices by customer or site, and label them with tags.</p></div>
      <div class="flex flex-wrap items-center gap-2">
        <button type="button" :disabled="saving || !store.abGuid" class="ui-button flex items-center gap-2" @click="editOrganization('group')"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.6" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M3 7v11a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-6l-2-3H5a2 2 0 0 0-2 2v1Zm9 4v6m-3-3h6" /></svg>New group</button>
        <button type="button" :disabled="saving || !store.abGuid" class="flex items-center gap-2 rounded-lg bg-rd-primary px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-rd-primary-hover disabled:opacity-50" @click="addPeer()"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.8" viewBox="0 0 24 24"><path stroke-linecap="round" d="M12 5v14M5 12h14" /></svg>Add device</button>
      </div>
    </div>
    <div class="grid items-stretch gap-4" style="grid-template-columns: repeat(auto-fit, minmax(min(100%, 16rem), 1fr))">
      <ServerOverview @changed="connection = $event" />
      <TagPanel :tags="store.tags" :selected-tag="store.selectedTag" :busy="saving || !store.abGuid" @select="selectTag" @create="editOrganization('tag')" @delete="removeTag" />
    </div>
    <ErrorNotice :message="actionError" />
    <ErrorNotice :message="store.error" retry @retry="store.init" />
    <div>
      <section aria-label="Device groups" class="min-w-0 flex-1">
        <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-2 text-xs text-gray-500 dark:text-rd-text-secondary"><span class="tabular-nums">{{ store.totalPeers }} devices</span><span aria-hidden="true">·</span><span>{{ store.groups.length }} groups</span><span v-if="store.loading" role="status" class="text-rd-primary">Updating…</span></div>
          <div class="flex max-w-full items-center gap-2"><button v-if="filtered" type="button" class="shrink-0 text-xs text-rd-primary" @click="clearFilters">Clear filters</button><div class="relative"><svg aria-hidden="true" class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-gray-400" fill="none" stroke="currentColor" stroke-width="1.7" viewBox="0 0 24 24"><path stroke-linecap="round" d="m21 21-5-5m2-6a7 7 0 1 1-14 0 7 7 0 0 1 14 0Z" /></svg><input v-model="search" type="search" maxlength="128" aria-label="Search devices" placeholder="Search devices…" class="peer-input w-full !pl-9 sm:w-64"></div></div>
        </div>
        <div v-if="store.overviewReady && !store.error" class="overflow-hidden rounded-xl border border-gray-200 bg-white dark:border-rd-border dark:bg-rd-card">
          <div v-if="!store.totalPeers && !visibleGroups.length" class="flex flex-col items-center px-6 py-14 text-center"><span class="mb-4 flex size-12 items-center justify-center rounded-2xl bg-rd-primary/5 text-rd-primary"><svg aria-hidden="true" class="size-6" fill="none" stroke="currentColor" stroke-width="1.4" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M4 4h16v12H4zM8 20h8m-4-4v4" /></svg></span><h2 class="text-sm font-semibold">{{ filtered ? 'No matching devices' : 'Your devices, organized' }}</h2><p class="mt-2 max-w-sm text-sm leading-relaxed text-gray-500 dark:text-rd-text-secondary">{{ filtered ? 'Try a different search or clear the tag filter.' : 'Create groups for your customers or sites, then add devices and assign tags.' }}</p><button type="button" :disabled="saving" class="mt-5 text-sm font-medium text-rd-primary" @click="filtered ? clearFilters() : addPeer()">{{ filtered ? 'Clear filters' : 'Add your first device' }}</button></div>
          <div class="divide-y divide-gray-200 dark:divide-rd-border">
            <PeerGroupSection v-if="store.ungroupedCount > 0 || (!filtered && store.groups.length > 0)" :key="`${store.abGuid}:ungrouped`" :guid="store.abGuid" :group="null" :count="store.ungroupedCount" :tags="store.tags" :query="store.query" :selected-tag="store.selectedTag" :connection="connection" :refresh-key="store.refreshKey" :initial-open="store.ungroupedCount > 0" :busy="saving" @add="addPeer" @edit="editPeer" @remove="removePeer" />
            <PeerGroupSection v-for="group in visibleGroups" :key="`${store.abGuid}:${group.id}`" :guid="store.abGuid" :group="group" :count="group.peer_count" :tags="store.tags" :query="store.query" :selected-tag="store.selectedTag" :connection="connection" :refresh-key="store.refreshKey" :initial-open="store.ungroupedCount === 0 && group.id === firstGroupId" :busy="saving" @add="addPeer" @edit="editPeer" @remove="removePeer" @edit-group="editOrganization('group', $event)" @remove-group="removeGroup" />
          </div>
        </div>
        <div v-else-if="store.loading" role="status" class="rounded-xl border border-gray-200 p-10 text-center text-sm text-gray-400 dark:border-rd-border">Loading device groups…</div>
      </section>
    </div>
  </div>
  <AddPeerDialog v-if="peerEditor" :peer="peerEditor.peer" :initial-group="peerEditor.groupId" :initial-tags="store.selectedTag ? [store.selectedTag] : []" :groups="store.groups" :tags="store.tags" :create-tag="store.addTag" :busy="saving" :server-error="actionError" @close="peerEditor = null" @save="savePeer" />
  <OrganizationDialog v-if="organization" :kind="organization.kind" :group="organization.group" :busy="saving" :error="actionError" @close="organization = null" @save="saveOrganization" />
</template>
