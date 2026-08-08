import { state, filters } from '../state'
import { el, fmtUsd, pct, fmtAmount } from '../format'
import { countNumber } from '../effects'
import { renderRouteBadges, poolName, type Analysis } from '../analysis'

const rowBps = new Map<string, number>()

let listEl: HTMLElement
let countEl: HTMLElement
let toolbarEl: HTMLElement

function filteredList(): Analysis[] {
  const all = [...state.analyses.values()]
  return all
    .filter((a) => {
      if (!a.opp.legs.every((l) => filters.poolOn.has(l.pool_idx)))
        return false
      if (!a.opp.profitable) return filters.showNearMiss
      if (a.netBps < filters.minRoiBps) return false
      if (a.confidence < filters.confMin) return false
      if (a.maxSizeUsd < filters.minLiqUsd) return false
      return true
    })
    .sort(
      (x, y) =>
        Number(y.opp.profitable) - Number(x.opp.profitable) ||
        y.netBps - x.netBps,
    )
}

function emptyFeed(): HTMLElement {
  const box = el('div', 'empty')
  box.appendChild(
    el(
      'p',
      'empty-title',
      'No edges right now',
    ),
  )
  box.appendChild(
    el(
      'p',
      'empty-sub',
      `Watching ${state.pools.length} pools across ${state.tokens.length} tokens. When three of them disagree by more than ${state.minProfitBps} bp net (or ${state.reportMinGrossBps} bp gross, fee-eaten), the route will show up here.`,
    ),
  )
  return box
}

function renderFilters(): void {
  toolbarEl.replaceChildren()

  const mkSelect = (
    label: string,
    value: string,
    onChange: (v: string) => void,
    options: [string, string][],
  ): void => {
    const g = el('label', 'filter-group')
    g.appendChild(el('span', 'lbl', label))
    const sel = el('select')
    for (const [val, txt] of options) {
      const o = el('option', undefined, txt)
      o.value = val
      sel.appendChild(o)
    }
    sel.value = value
    sel.addEventListener('change', () => onChange(sel.value))
    g.appendChild(sel)
    toolbarEl.appendChild(g)
  }

  mkSelect(
    'Min ROI',
    String(filters.minRoiBps),
    (v) => {
      filters.minRoiBps = Number(v)
      renderFeed()
    },
    [['0', '0 bp'], ['50', '≥ 50 bp'], ['100', '≥ 100 bp'], ['200', '≥ 200 bp']],
  )
  mkSelect(
    'Capacity',
    String(filters.minLiqUsd),
    (v) => {
      filters.minLiqUsd = Number(v)
      renderFeed()
    },
    [['0', 'any'], ['5000', '≥ $5k'], ['50000', '≥ $50k'], ['200000', '≥ $200k']],
  )
  mkSelect(
    'Confidence',
    String(filters.confMin),
    (v) => {
      filters.confMin = Number(v)
      renderFeed()
    },
    [['0', 'any'], ['25', '≥ 25%'], ['50', '≥ 50%'], ['75', '≥ 75%']],
  )

  const nm = el(
    'button',
    `nm-toggle${filters.showNearMiss ? ' on' : ''}`,
    filters.showNearMiss ? 'Near-miss: on' : 'Near-miss: off',
  )
  nm.type = 'button'
  nm.addEventListener('click', () => {
    filters.showNearMiss = !filters.showNearMiss
    renderFilters()
    renderFeed()
  })
  toolbarEl.appendChild(nm)

  for (let i = 0; i < state.pools.length; i++) {
    const chip = el('button', 'pool-chip', `#${i} ${poolName(state.pools[i])}`)
    chip.type = 'button'
    if (!filters.poolOn.has(i)) chip.classList.add('off')
    chip.addEventListener('click', () => {
      if (filters.poolOn.has(i)) filters.poolOn.delete(i)
      else filters.poolOn.add(i)
      renderFilters()
      renderFeed()
    })
    toolbarEl.appendChild(chip)
  }
}

function refreshDetail(detail: HTMLElement, a: Analysis): void {
  const legs = el('div', 'dc-col')
  legs.appendChild(el('div', 'dc-title', 'Route steps'))
  for (const leg of a.legs) {
    const line = el(
      'div',
      `leg-line${leg.driver ? ' hot' : ''}`,
      `${leg.token_in} → ${leg.token_out}`,
    )
    line.appendChild(
      el(
        'span',
        'leg-fig',
        `${fmtAmount(leg.amount_in, state.decimals.get(leg.token_in) ?? 0)} → ${fmtAmount(leg.amount_out, state.decimals.get(leg.token_out) ?? 0)} · #${leg.pool_idx} · fee ${leg.fee_bps}bp · impact ${leg.impactPct.toFixed(2)}%`,
      ),
    )
    legs.appendChild(line)
  }

  const breakdown = el('div', 'dc-col')
  breakdown.appendChild(el('div', 'dc-title', 'Profit Breakdown'))
  const gross = Math.max(a.grossBps, a.netBps, 1)
  const bar = el('div', 'breakdown-bar')
  const mkSeg = (cls: string, bps: number) => {
    const s = el('span', `seg ${cls}`, undefined)
    s.style.width = `${Math.min(100, Math.max((bps / gross) * 100, 0))}%`
    return s
  }
  bar.append(mkSeg('net', a.netBps), mkSeg('fee', a.feeBps), mkSeg('impact', a.impactBps))
  breakdown.appendChild(bar)
  const legend = el('div', 'breakdown-legend')
  const lg = (cls: string, label: string) => {
    const s = el('span', 'lg')
    s.appendChild(el('span', `sw ${cls}`))
    s.appendChild(document.createTextNode(label))
    return s
  }
  legend.append(
    lg('net', `Net ${a.netBps >= 0 ? '+' : ''}${pct(a.netBps)}`),
    lg('fee', `Fees ${pct(-a.feeBps)}`),
    lg('impact', `Impact ${pct(-a.impactBps)}`),
  )
  breakdown.appendChild(legend)
  const rows = el('div', 'leg-table')
  const r = (k: string, v: string, cls?: string) => {
    const line = el('div', 'exec-row')
    line.appendChild(el('span', 'k', k))
    line.appendChild(el('span', `v${cls ? ' ' + cls : ''}`, v))
    rows.appendChild(line)
  }
  const netUnits = a.opp.end_amount - a.opp.start_amount
  r(
    'Net',
    `${netUnits >= 0 ? '+' : ''}${fmtAmount(netUnits, state.baseDec)} ${state.baseToken}`,
    netUnits >= 0 ? 'good' : 'bad',
  )
  r('Gross', `${a.grossBps >= 0 ? '+' : ''}${pct(a.grossBps)}`)
  r('Max size', `${a.maxSizeDisplay} ${state.baseToken}`)
  r('Impact', `${a.impactPctTotal.toFixed(2)}%`)
  breakdown.appendChild(rows)

  const why = el('div', 'dc-col')
  why.appendChild(el('div', 'dc-title', 'Why this happens'))
  const p = el('p', 'explainer')
  p.innerHTML = `<span class="tag">model</span>${a.explain}`
  why.appendChild(p)

  detail.replaceChildren(legs, breakdown, why)
}

function makeRow(a: Analysis): HTMLDetailsElement {
  const row = document.createElement('details')
  row.className = 'opp-row'
  row.classList.add('flash-appear')

  const sum = el('summary', 'opp-summary')
  const route = el('div', 'opp-route')
  route.appendChild(renderRouteBadges(a.opp.path))
  const legsTxt = el(
    'span',
    'cell-k',
    `${a.opp.legs.length} hops · ${new Set(a.opp.legs.map((l) => l.pool_idx)).size} pools`,
  )
  route.appendChild(legsTxt)

  const profit = el('div', 'opp-profit num')
  const profitNum = el('span', 'profit-num', '—')
  profit.appendChild(profitNum)
  profit.appendChild(el('span', 'bps', `${a.netBps} bps`))
  profit.appendChild(el('span', 'closing-note', ''))

  const conf = el('div', 'opp-conf')
  const confPct = el('span', 'conf-pct num', `${a.confidence}%`)
  const mtr = el('div', `meter${a.confidence >= 50 ? ' good' : ''}`)
  const fill = el('span', undefined, undefined)
  mtr.appendChild(fill)
  conf.append(confPct, mtr)

  const meta = el('div', 'opp-meta num', '')
  const chev = el('span', 'opp-chev', '›')

  sum.append(route, profit, conf, meta, chev)
  row.appendChild(sum)

  const detail = el('div', 'opp-detail')
  detail.className = 'opp-detail'
  row.appendChild(detail)

  sum.addEventListener('toggle', () => {
    if (row.open) state.expanded.add(a.key)
    else state.expanded.delete(a.key)
  })

  return row
}

function refreshRow(row: HTMLDetailsElement, a: Analysis): void {
  row.classList.toggle('near-miss', !a.opp.profitable)
  const profitNum = row.querySelector('.profit-num') as HTMLElement
  const prev = rowBps.get(a.key)
  const fmtSigned = (v: number) => `${v >= 0 ? '+' : ''}${(v / 100).toFixed(2)}%`
  if (prev !== a.netBps) {
    countNumber(profitNum, prev ?? 0, a.netBps, fmtSigned, 500)
    rowBps.set(a.key, a.netBps)
  }
  const bps = row.querySelector('.bps') as HTMLElement
  if (bps)
    bps.textContent = a.opp.profitable
      ? `${a.netBps} bps`
      : `gross ${a.grossBps >= 0 ? '+' : ''}${a.grossBps} bps`
  const note = row.querySelector('.closing-note') as HTMLElement
  if (note) note.textContent = a.opp.profitable ? '' : 'fee-eaten'
  const fill = row.querySelector('.meter > span') as HTMLElement
  if (fill) fill.style.width = `${a.confidence}%`
  const confPct = row.querySelector('.conf-pct') as HTMLElement
  if (confPct) confPct.textContent = `${a.confidence}%`
  const meta = row.querySelector('.opp-meta') as HTMLElement
  if (meta)
    meta.textContent = `max ${fmtUsd(a.maxSizeUsd)} · impact ${a.impactPctTotal.toFixed(2)}%`

  const detail = row.querySelector('.opp-detail') as HTMLElement
  if (row.open) refreshDetail(detail, a)

  if (state.expanded.has(a.key) && !row.open) row.open = true
}

export function renderFeed(): void {
  const list = filteredList()
  countEl.textContent = list.length > 0 ? `${list.length} live` : '0 live'

  const closing = new Map(state.vanished.map((v) => [v.key, v]))

  const rowsById = new Map<string, HTMLDetailsElement>()
  listEl.querySelectorAll<HTMLDetailsElement>('.opp-row').forEach((r) => {
    rowsById.set(r.dataset.key ?? '', r)
  })

  if (list.length === 0 && closing.size === 0) {
    listEl.replaceChildren(emptyFeed())
    return
  }

  const frag = document.createDocumentFragment()
  const appended = new Set<string>()

  for (const a of list) {
    let row = rowsById.get(a.key)
    if (!row) {
      row = makeRow(a)
      row.dataset.key = a.key
      rowBps.set(a.key, 0)
    }
    row.classList.remove('closing')
    refreshRow(row, a)
    frag.appendChild(row)
    appended.add(a.key)
  }

  for (const [key, v] of closing) {
    const row = rowsById.get(key)
    if (!row || appended.has(key)) continue
    row.classList.add('closing')
    const note = row.querySelector('.closing-note') as HTMLElement
    if (note) note.textContent = `closed — was +${pct(v.bps)}`
    frag.appendChild(row)
    appended.add(key)
  }

  for (const [key, row] of rowsById) {
    if (!appended.has(key)) {
      row.remove()
      rowBps.delete(key)
    }
  }

  listEl.replaceChildren(frag)
}

export function renderFeedShell(): HTMLElement {
  const module = el('section', 'module module-feed')
  module.dataset.animate = ''
  const head = el('header', 'module-head')
  head.innerHTML =
    `<h2 class="module-title"><span class="live-dot"></span>Live edges</h2>`
  countEl = el('span', 'module-meta', '—')
  head.appendChild(countEl)
  module.appendChild(head)

  toolbarEl = el('div', 'feed-toolbar')
  module.appendChild(toolbarEl)
  listEl = el('div', 'feed-list')
  module.appendChild(listEl)
  renderFilters()
  renderFeed()
  return module
}
