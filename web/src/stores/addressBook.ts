import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as abApi from '../api/addressBook'

export const useAddressBookStore = defineStore('addressBook', () => {
  const abGuid = ref('')
  const peers = ref<abApi.Peer[]>([])
  const tags = ref<abApi.Tag[]>([])
  const totalPeers = ref(0)
  const selectedTag = ref<string | null>(null)
  const query = ref('')
  const page = ref(1)
  const pageSize = 24
  const loading = ref(false)
  const error = ref('')
  let activeRequest: AbortController | undefined

  async function init() {
    error.value = ''
    try {
      const res = await abApi.getPersonalAb()
      abGuid.value = res.data.guid
      await Promise.all([loadPeers(), loadTags()])
    } catch (e) { error.value = e instanceof Error ? e.message : 'Unable to load address book' }
  }
  async function loadPeers() {
    if (!abGuid.value) return
    activeRequest?.abort()
    const request = new AbortController()
    activeRequest = request
    loading.value = true
    error.value = ''
    try {
      const res = await abApi.getPeers(abGuid.value, page.value, pageSize, query.value, selectedTag.value || '', request.signal)
      if (request.signal.aborted) return
      peers.value = res.data
      totalPeers.value = res.total
      const lastPage = Math.max(1, Math.ceil(res.total / pageSize))
      if (page.value > lastPage) { page.value = lastPage; await loadPeers() }
    } catch (e) {
      if (!request.signal.aborted) error.value = e instanceof Error ? e.message : 'Unable to load devices'
    } finally { if (activeRequest === request) loading.value = false }
  }
  async function loadTags() {
    const res = await abApi.getTags(abGuid.value)
    tags.value = res.data
  }
  async function setPage(next: number) { page.value = next; await loadPeers() }
  async function setFilters(q: string, tag: string | null) {
    query.value = q; selectedTag.value = tag; page.value = 1; await loadPeers()
  }
  async function addPeer(peer: Partial<abApi.Peer>) { await abApi.addPeer(abGuid.value, peer); await loadPeers() }
  async function removePeer(ids: string[]) { await abApi.deletePeer(abGuid.value, ids); await loadPeers() }
  async function addTag(name: string, color: number) { await abApi.addTag(abGuid.value, name, color); await loadTags() }
  async function removeTag(names: string[]) {
    await abApi.deleteTag(abGuid.value, names)
    if (names.includes(selectedTag.value || '')) selectedTag.value = null
    await Promise.all([loadTags(), loadPeers()])
  }
  function reset() {
    activeRequest?.abort(); abGuid.value = ''; peers.value = []; tags.value = []
    totalPeers.value = 0; selectedTag.value = null; query.value = ''; page.value = 1
    error.value = ''; loading.value = false
  }
  return { abGuid, peers, tags, totalPeers, selectedTag, query, page, pageSize, loading, error,
    init, loadPeers, loadTags, setPage, setFilters, addPeer, removePeer, addTag, removeTag, reset }
})
