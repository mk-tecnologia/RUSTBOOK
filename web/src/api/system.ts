import { api } from './client'

export async function getServerVersion(): Promise<string> {
  const response = await api<{ version: unknown }>('/api/version', {
    cache: 'no-store',
    signal: AbortSignal.timeout(5000),
  })
  if (typeof response.version !== 'string' || !response.version) {
    throw new Error('Server version unavailable')
  }
  return response.version
}
