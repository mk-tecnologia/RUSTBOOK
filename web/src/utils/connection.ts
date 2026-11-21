/** Open the configured native RustDesk client; never include credentials. */
export function rustdeskLink(id: string): string | null {
  const trimmed = id.trim()
  const normalized = /^[\d\s]+$/.test(trimmed) ? trimmed.replace(/\s/g, '') : trimmed
  if (!/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,127}$/.test(normalized)) return null
  return `rustdesk://connect/${encodeURIComponent(normalized)}`
}
