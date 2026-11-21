import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as abApi from '../api/addressBook'

export const useAddressBookStore = defineStore('addressBook', () => {
  const abGuid = ref('')
  const groups = ref<abApi.PeerGroup[]>([])
  const tags = ref<abApi.Tag[]>([])
  const totalPeers = ref(0)
  const ungroupedCount = ref(0)
  const selectedTag = ref<string | null>(null)
  const query = ref('')
  const refreshKey = ref(0)
  const loading = ref(false)
  const error = ref('')
  const overviewReady = ref(false)
  let activeRequest: AbortController | undefined
  let generation = 0

  async function init() {
    const current = generation
    error.value = ''
    loading.value = true
    try {
      const res = await abApi.getPersonalAb()
      if (generation !== current) return
      abGuid.value = res.data.guid
      await Promise.all([loadOverview(), loadTags()])
    } catch (e) {
      if (generation === current) error.value = e instanceof Error ? e.message : 'Unable to load address book'
    } finally { if (generation === current && !activeRequest) loading.value = false }
  }
  async function loadOverview() {
    if (!abGuid.value) return
    activeRequest?.abort()
    const request = new AbortController()
    activeRequest = request
    loading.value = true
    error.value = ''
    try {
      const res = await abApi.getPeerGroups(abGuid.value, query.value, selectedTag.value || '', request.signal)
      if (request.signal.aborted) return
      groups.value = res.data
      totalPeers.value = res.total
      ungroupedCount.value = res.ungrouped_count
      overviewReady.value = true
    } catch (e) {
      if (!request.signal.aborted) error.value = e instanceof Error ? e.message : 'Unable to load devices'
    } finally {
      if (activeRequest === request) { loading.value = false; activeRequest = undefined }
    }
  }
  async function loadTags() {
    const guid = abGuid.value
    const current = generation
    const res = await abApi.getTags(guid)
    if (guid === abGuid.value && current === generation) tags.value = res.data
  }
  async function setFilters(q: string, tag: string | null) {
    query.value = q; selectedTag.value = tag
    await loadOverview()
  }
  async function refresh() { refreshKey.value++; await loadOverview() }
  async function addPeer(peer: abApi.PeerForm) { await abApi.addPeer(abGuid.value, peer); await refresh() }
  async function updatePeer(peer: abApi.PeerForm) { await abApi.updatePeer(abGuid.value, peer); await refresh() }
  async function removePeer(ids: string[]) { await abApi.deletePeer(abGuid.value, ids); await refresh() }
  async function addTag(name: string, color: number) { await abApi.addTag(abGuid.value, name, color); await loadTags() }
  async function removeTag(names: string[]) {
    await abApi.deleteTag(abGuid.value, names)
    if (names.includes(selectedTag.value || '')) selectedTag.value = null
    await Promise.all([loadTags(), refresh()])
  }
  async function saveGroup(group: { id?: number; name: string; note: string }) {
    if (group.id !== undefined) await abApi.updatePeerGroup(abGuid.value, group.id, group)
    else await abApi.createPeerGroup(abGuid.value, group)
    await refresh()
  }
  async function removeGroup(id: number) { await abApi.deletePeerGroup(abGuid.value, id); await refresh() }
  function reset() {
    generation++; activeRequest?.abort(); activeRequest = undefined
    abGuid.value = ''; groups.value = []; tags.value = []; totalPeers.value = 0; ungroupedCount.value = 0
    selectedTag.value = null; query.value = ''; error.value = ''; loading.value = false; refreshKey.value++
    overviewReady.value = false
  }
  return { abGuid, groups, tags, totalPeers, ungroupedCount, selectedTag, query, refreshKey, loading, error, overviewReady,
    init, loadOverview, loadTags, setFilters, refresh, addPeer, updatePeer, removePeer, addTag,
    removeTag, saveGroup, removeGroup, reset }
})
