import type { UserItem } from '../api/addressBook'

export interface UserDraft {
  username: string; name: string; email: string; is_admin: boolean; status: number
  password: string; confirmation: string
}
export function userDraft(user?: UserItem): UserDraft {
  return { username: user?.username || '', name: user?.name || '', email: user?.email || '',
    is_admin: user?.is_admin ?? false, status: user?.status ?? 1, password: '', confirmation: '' }
}
export function userPayload(draft: UserDraft, original?: UserItem) {
  if (!draft.username.trim()) throw new Error('Username is required.')
  if (!original || draft.password || draft.confirmation) {
    const bytes = new TextEncoder().encode(draft.password).length
    if (bytes < 12 || bytes > 1024) throw new Error('Password must contain 12–1024 bytes.')
    if (draft.password !== draft.confirmation) throw new Error('Passwords do not match.')
  }
  const profile = { name: draft.name.trim(), email: draft.email.trim(), is_admin: draft.is_admin }
  if (!original) return { ...profile, username: draft.username.trim(), password: draft.password }
  // Omit an unchanged status: sending it unnecessarily revokes active sessions.
  return { ...profile, ...(draft.status !== original.status ? { status: draft.status } : {}),
    ...(draft.password ? { password: draft.password } : {}) }
}
