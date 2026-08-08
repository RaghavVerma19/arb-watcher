import { state } from '../state'
import { el, svgEl, fmtUsd } from '../format'
import { poolName } from '../analysis'
import type { Pool } from '../types'

let grid: HTMLElement
let liqBody: HTMLElement
let meta: HTMLElement
let tokenOrder: string[] = []
let prevTickPrices = new Map<string, number>()

interface SparkRefs {
  area: SVGPathElement
  line: SVGPathElement
}

interface CardRefs {
  card: HTMLElement
  dlt: HTMLElement
  price: HTMLElement
  spark: SparkRefs
  mvs: HTMLElement[]
}

interface LiqRefs {
  a: HTMLElement
  b: HTMLElement
  left: HTMLElement
  right: HTMLElement
}

const cards = new Map<string, CardRefs>()
const liqRefs: LiqRefs[] = []

function buildSpark(): { svg: SVGSVGElement; area: SVGPathElement; line: SVGPathElement } {
  const w = 180
  const h = 40
  const svg = svgEl('svg', { viewBox: `0 0 ${w} ${h}`, class: 'intel-spark' })
  svg.setAttribute('aria-hidden', 'true')
  const area = svgEl('path', {})
  area.style.fill = 'var(--color-profit)'
  area.setAttribute('opacity', '0.08')
  svg.appendChild(area)
  const line = svgEl('path', { fill: 'none' })
  line.style.stroke = 'var(--color-profit)'
  line.setAttribute('stroke-width', '1.4')
  line.setAttribute('stroke-linejoin', 'round')
  svg.appendChild(line)
  return { svg, area, line }
}

function updateSpark(refs: SparkRefs, data: number[]): void {
  const w = 180
  const h = 40
  const clean = data.filter((v) => Number.isFinite(v) && v > 0)
  if (clean.length < 2) {
    refs.area.setAttribute('d', '')
    refs.line.setAttribute('d', '')
    return
  }
  const min = Math.min(...clean)
  const max = Math.max(...clean)
  const span = max - min || 1
  const pad = 4
  const pts = clean.map((v, i) => {
    const x = pad + (i / (clean.length - 1)) * (w - pad * 2)
    const y = pad + (1 - (v - min) / span) * (h - pad * 2)
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })
  const up = data[data.length - 1] >= data[0]
  const stroke = up ? 'var(--color-profit)' : 'var(--color-loss)'
  refs.area.setAttribute(
    'd',
    `M${pts.join(' L')} L${w - pad},${h - pad} L${pad},${h - pad} Z`,
  )
  refs.area.style.fill = stroke
  refs.line.setAttribute('d', `M${pts.join(' L')}`)
  refs.line.style.stroke = stroke
}

function priceFmt(price: number): string {
  return price.toLocaleString(undefined, {
    minimumFractionDigits: price >= 10 ? 2 : 4,
    maximumFractionDigits: price >= 10 ? 2 : 4,
  })
}

function buildCard(sym: string): CardRefs {
  const card = el('div', 'intel-card')
  card.dataset.sym = sym

  const top = el('div', 'intel-top')
  top.appendChild(el('span', 'sym', sym))
  const dlt = el('span', 'dlt', '—')
  top.appendChild(dlt)
  card.appendChild(top)

  const price = el('div', 'intel-price num', '—')
  card.appendChild(price)

  const spark = buildSpark()
  card.appendChild(spark.svg)

  const metaGrid = el('div', 'intel-meta')
  const mvs: HTMLElement[] = []
  const mi = (k: string) => {
    const s = el('div')
    s.appendChild(el('span', 'cell-k', k))
    const mv = el('span', 'mv num', '—')
    s.appendChild(mv)
    mvs.push(mv)
    return s
  }
  metaGrid.append(mi('Liquidity'), mi('Pools'), mi('Session'), mi('Volatility'))
  card.appendChild(metaGrid)

  grid.appendChild(card)
  return { card, dlt, price, spark, mvs }
}

function tokenLiquidityUsd(sym: string): number {
  let total = 0
  for (const p of state.pools) {
    if (p.token_a === sym || p.token_b === sym) {
      const da = state.decimals.get(p.token_a) ?? 0
      const db = state.decimals.get(p.token_b) ?? 0
      const pa = state.lastPrices.get(p.token_a) ?? 0
      const pb = state.lastPrices.get(p.token_b) ?? 0
      const va = (p.reserve_a / 10 ** da) * pa
      const vb = (p.reserve_b / 10 ** db) * pb
      total += Math.min(va, vb)
    }
  }
  return total
}

function volatilityPct(hist: number[]): number {
  if (hist.length < 3) return 0
  const deltas: number[] = []
  for (let i = 1; i < hist.length; i++) {
    if (hist[i - 1] > 0) deltas.push((hist[i] - hist[i - 1]) / hist[i - 1])
  }
  if (deltas.length === 0) return 0
  const mean = deltas.reduce((s, d) => s + d, 0) / deltas.length
  const variance = deltas.reduce((s, d) => s + (d - mean) ** 2, 0) / deltas.length
  return Math.sqrt(variance) * 100
}

export function renderIntelShell(): HTMLElement {
  const module = el('section', 'module module-intel')
  module.dataset.animate = ''
  const head = el('header', 'module-head')
  head.innerHTML = `<h2 class="module-title">Price snapshots</h2>`
  meta = el('span', 'module-meta', '—')
  head.appendChild(meta)
  module.appendChild(head)
  grid = el('div', 'intel-grid')
  module.appendChild(grid)
  const liqHead = el('h3', 'intel-subhead', 'Pool depth')
  module.appendChild(liqHead)
  liqBody = el('div', 'liq-list')
  module.appendChild(liqBody)
  renderIntel()
  renderLiquidity()
  return module
}

export function renderIntel(): void {
  if (!grid) return
  const tokens = state.tokens

  if (tokenOrder.join('|') !== tokens.map((t) => t.symbol).join('|')) {
    tokenOrder = [...tokens].map((t) => t.symbol).sort()
    for (const sym of tokenOrder) {
      if (!cards.has(sym)) cards.set(sym, buildCard(sym))
    }
    for (const [sym, ref] of cards) {
      if (!tokenOrder.includes(sym)) {
        ref.card.remove()
        cards.delete(sym)
      }
    }
  }

  meta.textContent = `${tokens.length} ${tokens.length === 1 ? 'token' : 'tokens'} · updating live`

  for (const sym of tokenOrder) {
    const ref = cards.get(sym)
    if (!ref) continue
    const price = state.lastPrices.get(sym)
    if (price === undefined) continue

    ref.price.textContent = priceFmt(price)

    const prev = prevTickPrices.get(sym)
    if (prev !== undefined && prev !== price) {
      const up = price >= prev
      const d = Math.abs(((price - prev) / prev) * 100)
      ref.dlt.textContent = `${up ? '▲' : '▼'} ${d.toFixed(2)}%`
      ref.dlt.style.color = up ? 'var(--color-profit)' : 'var(--color-loss)'
    } else {
      ref.dlt.textContent = '—'
      ref.dlt.style.color = 'var(--color-muted)'
    }

    const hist = state.priceHistory.get(sym) ?? []
    updateSpark(ref.spark, hist)

    const first = hist[0]
    const sessPct = first !== undefined ? ((price - first) / first) * 100 : 0
    const dexCount = state.pools.filter(
      (p) => p.token_a === sym || p.token_b === sym,
    ).length
    const vol = volatilityPct(hist)
    ref.mvs[0].textContent = fmtUsd(tokenLiquidityUsd(sym))
    ref.mvs[1].textContent = String(dexCount)
    ref.mvs[2].textContent = `${sessPct >= 0 ? '+' : ''}${sessPct.toFixed(1)}%`
    ref.mvs[3].textContent = `${vol.toFixed(1)}%`
  }
}

function buildLiqRow(p: Pool, i: number): LiqRefs {
  const row = el('div', 'liq-row')
  row.dataset.idx = String(i)

  const top = el('div', 'liq-top')
  top.appendChild(el('span', 'liq-name', poolName(p)))
  top.appendChild(el('span', 'liq-fee num', `${p.fee_bps}bp`))
  row.appendChild(top)

  const bar = el('div', 'depthbar')
  const a = el('span', 'side-a', undefined)
  const b = el('span', 'side-b', undefined)
  bar.append(a, b)
  row.appendChild(bar)

  const sub = el('div', 'liq-sub num')
  const left = el('span', 'lv', '—')
  const right = el('span', 'rv', '—')
  sub.append(left, right)
  row.appendChild(sub)

  liqBody.appendChild(row)
  return { a, b, left, right }
}

export function renderLiquidity(): void {
  if (!liqBody) return
  const pools = state.pools
  if (pools.length === 0) return

  while (liqRefs.length < pools.length) {
    const i = liqRefs.length
    const p = pools[i]
    liqRefs.push(buildLiqRow(p, i))
  }
  while (liqRefs.length > pools.length) {
    const ref = liqRefs.pop()
    ref?.a.closest('.liq-row')?.remove()
  }

  const depths = pools.map((p) => {
    const da = state.decimals.get(p.token_a) ?? 0
    const db = state.decimals.get(p.token_b) ?? 0
    const pa = state.lastPrices.get(p.token_a) ?? 0
    const pb = state.lastPrices.get(p.token_b) ?? 0
    return {
      va: (p.reserve_a / 10 ** da) * pa,
      vb: (p.reserve_b / 10 ** db) * pb,
    }
  })
  const maxDepth = Math.max(...depths.map((d) => d.va + d.vb), 1)

  pools.forEach((p, i) => {
    const ref = liqRefs[i]
    if (!ref) return
    const d = depths[i]
    const total = d.va + d.vb
    const imbalance = total > 0 ? Math.abs(d.va - d.vb) / total : 0
    ref.a.style.width = `${(d.va / maxDepth) * 100}%`
    ref.b.style.width = `${(d.vb / maxDepth) * 100}%`
    ref.left.textContent = `${p.token_a} ${fmtUsd(d.va)}`
    if (imbalance > 0.2) {
      ref.right.textContent = `lopsided — ${(imbalance * 100).toFixed(0)}% imbalance`
      ref.right.className = 'imbalance'
    } else {
      ref.right.textContent = `${fmtUsd(total)} total`
      ref.right.className = 'rv'
    }
  })
}

export function tickIntelBaseline(): void {
  prevTickPrices = state.lastPrices
}
