import { api } from './client'

export interface ServerVersion {
  version: string
  commit: string | null
}

export async function getServerVersion(): Promise<ServerVersion> {
  const response = await api<{ version: unknown; commit?: unknown }>('/api/version', {
    cache: 'no-store',
    signal: AbortSignal.timeout(5000),
  })
  if (typeof response.version !== 'string' || !response.version) {
    throw new Error('Server version unavailable')
  }
  return {
    version: response.version,
    commit: typeof response.commit === 'string' && /^[a-f0-9]{40}$/.test(response.commit)
      ? response.commit : null,
  }
}
