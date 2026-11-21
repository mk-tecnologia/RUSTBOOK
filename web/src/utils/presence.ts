import type { Peer } from '../api/addressBook'

export function presenceSummary(peer: Pick<Peer, 'online' | 'presence_source' | 'last_seen'>) {
  let label = 'Unknown'
  let tone: 'online' | 'offline' | 'recent' | 'unknown' = 'unknown'
  let description = 'No live presence confirmation is available.'
  if (peer.presence_source === 'hbbs' && typeof peer.online === 'boolean') {
    label = peer.online ? 'Online' : 'Offline'
    tone = peer.online ? 'online' : 'offline'
    description = 'Presence confirmed by the configured RustDesk ID server (hbbs).'
  } else if (peer.presence_source === 'api_report' && peer.online === true) {
    label = 'Recent activity'; tone = 'recent'
    description = 'The API received a recent report. Live presence on the ID server has not been confirmed.'
  } else if (peer.presence_source === 'probe_failed') {
    description = 'The ID server could not be queried. This does not mean the device is offline.'
  } else if (peer.presence_source === 'stale' || peer.online === false) {
    description = 'The last API report is old. An expired report does not confirm that the device is offline.'
  }
  if (peer.last_seen) {
    const timestamp = /^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d$/.test(peer.last_seen)
      ? peer.last_seen.replace(' ', 'T') + 'Z' : peer.last_seen
    const date = new Date(timestamp)
    if (Number.isFinite(date.getTime()) && date.getTime() <= Date.now()) {
      description += ` Last API report: ${date.toLocaleString()}.`
    }
  }
  return { label, tone, description }
}
