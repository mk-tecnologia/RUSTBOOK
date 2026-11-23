import type { ClientConfiguration } from '../api/serverSettings'
export const clientFields = [
  { key: 'id_server', label: 'Servidor de ID' },
  { key: 'relay_server', label: 'Servidor Relay' },
  { key: 'api_url', label: 'Servidor da API' },
  { key: 'public_key', label: 'Chave / Key pública' },
] as const
export function clientConfigurationText(config: ClientConfiguration): string {
  return clientFields.map(field => `${field.label}: ${config[field.key]}`).join('\n')
}
export async function copyText(value: string): Promise<boolean> {
  try { if (navigator.clipboard?.writeText) { await navigator.clipboard.writeText(value); return true } } catch { /* HTTP LAN can require the selection fallback. */ }
  const previous = document.activeElement as HTMLElement | null
  const input = document.createElement('textarea')
  input.value = value; input.setAttribute('readonly', '')
  input.style.cssText = 'position:fixed;left:0;top:0;opacity:0;pointer-events:none'
  document.body.append(input); input.focus({ preventScroll: true }); input.select(); input.setSelectionRange(0, input.value.length)
  let copied = false
  try { copied = document.execCommand('copy') } catch { /* Keep visible fields available for manual copying. */ }
  finally { input.remove(); previous?.focus({ preventScroll: true }) }
  return copied
}
