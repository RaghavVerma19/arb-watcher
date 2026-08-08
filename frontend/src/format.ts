export function fmtAmount(units: number, dec: number): string {
  return (units / 10 ** dec).toLocaleString(undefined, {
    maximumFractionDigits: dec,
  })
}

export function fmtBig(raw: bigint, dec: number): string {
  if (raw >= 10n ** 15n) {
    const v = 10 ** 15 / 10 ** dec
    return `≥${v.toLocaleString(undefined, { maximumFractionDigits: 0 })}`
  }
  return fmtAmount(Number(raw), dec)
}

export function fmtUsd(x: number): string {
  if (x >= 1e9) return `$${(x / 1e9).toFixed(2)}B`
  if (x >= 1e6) return `$${(x / 1e6).toFixed(2)}M`
  if (x >= 1e3) return `$${(x / 1e3).toFixed(1)}k`
  return `$${x.toFixed(0)}`
}

export function pct(bps: number): string {
  return (bps / 100).toFixed(2) + '%'
}

export function clamp(x: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, x))
}

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag)
  if (className) node.className = className
  if (text !== undefined) node.textContent = text
  return node
}

export function svgEl<K extends keyof SVGElementTagNameMap>(
  tag: K,
  attrs: Record<string, string | number>,
): SVGElementTagNameMap[K] {
  const node = document.createElementNS('http://www.w3.org/2000/svg', tag)
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, String(v))
  return node
}
