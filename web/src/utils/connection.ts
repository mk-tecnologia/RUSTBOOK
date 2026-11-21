export interface NativeConnection { server: string; public_key: string }
/** Only the public server key is included; never remote passwords or account tokens. */
export function rustdeskLink(id: string, connection?: NativeConnection): string | null {
  const trimmed = id.trim()
  const normalized = /^[\d\s]+$/.test(trimmed) ? trimmed.replace(/\s/g, '') : trimmed
  if (!/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,127}$/.test(normalized)) return null
  const base = `rustdesk://connect/${encodeURIComponent(normalized)}`
  if (!connection?.server && !connection?.public_key) return base
  // Explicit server connections in RustDesk avoid forwarding the API account token
  // to hbbs OSS. Always retain the administrator-provided public verification key.
  if (!connection.server || !/^[A-Za-z0-9+/]{43}=$/.test(connection.public_key)) return null
  try {
    const authority = new URL(`http://${connection.server}`)
    if (!authority.hostname || !/:\d+$/.test(connection.server) || Number(connection.server.match(/:(\d+)$/)?.[1]) < 1 || authority.username || authority.password
      || authority.pathname !== '/' || authority.search || authority.hash
      || /[\s/?#@]/.test(connection.server)) return null
  } catch { return null }
  return `${base}@${connection.server}?key=${encodeURIComponent(connection.public_key)}`
}
