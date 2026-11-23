import type { Tag } from '../api/addressBook'
function folded(value: string) { return value.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLocaleLowerCase() }
export function filterTags(tags: Tag[], query: string): Tag[] {
  const terms = folded(query).trim().split(/\s+/).filter(Boolean)
  return tags.filter(tag => { const name = folded(tag.name); return terms.every(term => name.includes(term)) })
}
export function selectTag(current: string[], name: string, multiple: boolean): string[] {
  if (!multiple) return name ? [name] : []
  return current.includes(name) ? current.filter(item => item !== name) : [...current, name]
}

export function tagPopupPlacement(
  rect: { left: number; top: number; bottom: number; width: number },
  viewport: { left: number; top: number; width: number; height: number },
  contentHeight: number,
) {
  const edge = 8
  const bottom = viewport.top + viewport.height
  const below = Math.max(0, bottom - rect.bottom - edge)
  const above = Math.max(0, rect.top - viewport.top - edge)
  const down = below >= 240 || below >= above
  const width = Math.max(0, Math.min(rect.width, viewport.width - edge * 2))
  const maxHeight = Math.max(0, Math.min(340, down ? below : above, viewport.height - edge * 2))
  const height = Math.min(contentHeight, maxHeight)
  return {
    left: Math.max(viewport.left + edge, Math.min(rect.left, viewport.left + viewport.width - width - edge)),
    top: Math.max(viewport.top + edge, Math.min(down ? rect.bottom + 6 : rect.top - height - 6, bottom - height - edge)),
    width, maxHeight,
  }
}
