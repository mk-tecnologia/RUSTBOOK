import { api } from './client'

export interface Peer {
  id: string
  hash: string
  username: string
  hostname: string
  platform: string
  alias: string
  tags: string[]
  note: string
  group_id?: number | null
  online?: boolean
  presence_source?: 'hbbs' | 'api_report' | 'stale' | 'unavailable' | 'probe_failed'
  last_seen?: string
}

export interface PeerGroup {
  id: number
  name: string
  note: string
  peer_count: number
}

export interface GroupOverview {
  data: PeerGroup[]
  total: number
  ungrouped_count: number
}

export interface PeerForm {
  id: string
  alias: string
  hostname: string
  username: string
  platform: string
  note: string
  tags: string[]
  group_id: number | null
}

export interface Tag {
  name: string
  color: number
}

export interface AbProfile {
  guid: string
  name: string
  owner: string
  rule: number
  note: string
}

export interface PeersResponse {
  data: Peer[]
  total: number
}

export interface TagsResponse {
  data: Tag[]
  total: number
}

export function getPersonalAb(): Promise<{ data: AbProfile }> {
  return api('/api/ab/personal')
}

export function getPeers(guid: string, page = 1, pageSize = 100, q = '', tag = '', signal?: AbortSignal, group = ''): Promise<PeersResponse> {
  const params = new URLSearchParams({ ab: guid, current: String(page), pageSize: String(pageSize), q, tag })
  if (group) params.set('group', group)
  return api(`/api/ab/peers?${params}`, { signal })
}

export function addPeer(guid: string, peer: Partial<Peer>): Promise<void> {
  return api(`/api/ab/peer/add/${guid}`, {
    method: 'POST',
    body: JSON.stringify({ ...peer, fail_if_exists: true }),
  })
}

export function updatePeer(guid: string, peer: Partial<Peer>): Promise<void> {
  return api(`/api/ab/peer/update/${guid}`, { method: 'PUT', body: JSON.stringify(peer) })
}

export function getPeerGroups(guid: string, q = '', tag = '', signal?: AbortSignal): Promise<GroupOverview> {
  return api(`/api/ab/peer-groups/${guid}?${new URLSearchParams({ q, tag })}`, { signal })
}

export function createPeerGroup(guid: string, group: { name: string; note: string }): Promise<{ id: number }> {
  return api(`/api/ab/peer-groups/${guid}`, { method: 'POST', body: JSON.stringify(group) })
}

export function updatePeerGroup(guid: string, id: number, group: { name: string; note: string }): Promise<void> {
  return api(`/api/ab/peer-groups/${guid}/${id}`, { method: 'PUT', body: JSON.stringify(group) })
}

export function deletePeerGroup(guid: string, id: number): Promise<void> {
  return api(`/api/ab/peer-groups/${guid}/${id}`, { method: 'DELETE' })
}

export function deletePeer(guid: string, ids: string[]): Promise<void> {
  return api(`/api/ab/peer/${guid}`, {
    method: 'DELETE',
    body: JSON.stringify({ ids }),
  })
}

export function getTags(guid: string): Promise<TagsResponse> {
  return api(`/api/ab/tags/${guid}`)
}

export function addTag(guid: string, name: string, color: number): Promise<void> {
  return api(`/api/ab/tag/add/${guid}`, {
    method: 'POST',
    body: JSON.stringify({ name, color }),
  })
}

export function renameTag(guid: string, oldName: string, newName: string): Promise<void> {
  return api(`/api/ab/tag/rename/${guid}`, {
    method: 'PUT',
    body: JSON.stringify({ old: oldName, new: newName }),
  })
}

export function updateTagColor(guid: string, name: string, color: number): Promise<void> {
  return api(`/api/ab/tag/update/${guid}`, {
    method: 'PUT',
    body: JSON.stringify({ name, color }),
  })
}

export function deleteTag(guid: string, names: string[]): Promise<void> {
  return api(`/api/ab/tag/${guid}`, {
    method: 'DELETE',
    body: JSON.stringify({ names }),
  })
}

// Admin APIs
export interface UserItem {
  id: number
  username: string
  name: string
  email: string
  is_admin: boolean
  status: number
  created_at: string
}

export interface GroupItem {
  id: number
  name: string
  note: string
  created_at: string
}

export function getUsers(): Promise<{ data: UserItem[]; total: number }> {
  return api('/api/users')
}

export function createUser(user: { username: string; password: string; name?: string; email?: string; is_admin?: boolean }): Promise<void> {
  return api('/api/users', { method: 'POST', body: JSON.stringify(user) })
}

export function updateUser(id: number, data: Record<string, unknown>): Promise<void> {
  return api(`/api/users/${id}`, { method: 'PUT', body: JSON.stringify(data) })
}

export function deleteUser(id: number): Promise<void> {
  return api(`/api/users/${id}`, { method: 'DELETE' })
}

export function getGroups(): Promise<{ data: GroupItem[]; total: number }> {
  return api('/api/groups')
}

export function createGroup(group: { name: string; note?: string }): Promise<void> {
  return api('/api/groups', { method: 'POST', body: JSON.stringify(group) })
}

export function updateGroup(id: number, data: Record<string, unknown>): Promise<void> {
  return api(`/api/groups/${id}`, { method: 'PUT', body: JSON.stringify(data) })
}

export function deleteGroup(id: number): Promise<void> {
  return api(`/api/groups/${id}`, { method: 'DELETE' })
}
