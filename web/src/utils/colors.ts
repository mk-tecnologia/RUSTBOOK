export function tagHex(color: number): string {
  return `#${((color >>> 0) & 0xffffff).toString(16).padStart(6, '0')}`
}
export function tagColor(hex: string): number {
  return (0xff000000 | Number.parseInt(hex.replace('#', ''), 16)) >>> 0
}
