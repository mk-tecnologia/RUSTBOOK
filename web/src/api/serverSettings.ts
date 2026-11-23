import { api } from './client'
export interface ServerSettings {
  id_server: string; relay_server: string; api_url: string; public_key: string
  presence_server: string; allow_private_relay: boolean; revision: string
}
export interface ClientConfiguration { id_server: string; relay_server: string; api_url: string; public_key: string; can_edit?: boolean }
export interface ServerReport {
  settings: ServerSettings; warnings: string[]
  agent: { connected?: boolean; running?: boolean; relay?: string; configured_relay?: string; public_key?: string; can_apply?: boolean; service?: string; compose_file?: string; error?: string }
  runtime: { port: number; token_expiry_hours: number; cors_origins: string[]; anonymous_device_events: boolean }
}
export const getServerSettings = () => api<ServerReport>('/api/admin/server-settings')
export const saveServerSettings = (settings: ServerSettings) => api<ServerReport>('/api/admin/server-settings', { method: 'PUT', body: JSON.stringify(settings) })
export const checkServerSettings = () => api<Record<string, string>>('/api/admin/server-settings/check', { method: 'POST' })
export const applyRelay = (revision: string) => api<{ ok: boolean; changed: boolean; restarted?: boolean; relay: string; keys_preserved: boolean; notice?: string }>('/api/admin/server-settings/apply-relay', { method: 'POST', body: JSON.stringify({ revision }) })
export const getClientConfiguration = () => api<ClientConfiguration>('/api/client-configuration')
export interface ServerHealth { report: ServerReport; checks: Record<string, string> }
export const getServerHealth = () => api<ServerHealth>('/api/admin/server-status')
