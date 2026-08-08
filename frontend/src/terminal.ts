import { state } from './state'
import { el } from './format'
import { reveal } from './effects'
import { poolName } from './analysis'
import { Sculpture, type HoverInfo } from './views/sculpture'
import { renderFeedShell, renderFeed } from './views/feed'
import { renderExecShell, renderExec } from './views/exec'
import { renderIntelShell, renderIntel, renderLiquidity, tickIntelBaseline } from './views/intel'
import { renderHistoryShell } from './views/history'

let sculpture: Sculpture | null = null
let hoverEl: HTMLElement
let tokenKey = ''
let canvas: HTMLCanvasElement
let terminalHistoryRender: (() => void) | null = null

function priceFmt(price: number): string {
  if (price === 0) return '—'
  return price.toLocaleString(undefined, {
    minimumFractionDigits: price >= 10 ? 2 : 4,
    maximumFractionDigits: price >= 10 ? 2 : 4,
  })
}

function onSculptHover(info: HoverInfo): void {
  if (!hoverEl) return
  if (!info) {
    hoverEl.textContent = 'drag to pan · scroll to zoom · double-click to reset'
    return
  }
  if (info.kind === 'node') {
    const price = state.lastPrices.get(info.sym)
    hoverEl.textContent =
      price !== undefined ? `${info.sym} · ${priceFmt(price)}` : info.sym
  } else {
    const pool = state.pools[info.poolIdx]
    const rate =
      pool && pool.reserve_a > 0
        ? pool.token_a === info.symA
          ? pool.reserve_b / pool.reserve_a
          : pool.reserve_a / pool.reserve_b
        : 0
    hoverEl.textContent = pool
      ? `${poolName(pool)} · #${info.poolIdx} · fee ${pool.fee_bps}bp · rate ${priceFmt(rate)}`
      : `${info.symA}/${info.symB}`
  }
}

export function renderTerminal(): void {
  const host = document.getElementById('view-terminal') as HTMLElement
  host.replaceChildren()
  const term = el('div', 'term')
  host.appendChild(term)

  const cols = el('div', 'term-cols')
  const main = el('div', 'term-main')
  main.appendChild(renderFeedShell())
  const historyShell = renderHistoryShell()
  terminalHistoryRender = historyShell.render
  main.appendChild(historyShell.module)
  cols.appendChild(main)
  cols.appendChild(renderExecShell())
  term.appendChild(cols)

  const support = el('div', 'term-support')
  const sculptModule = el('section', 'module module-sculpt')
  sculptModule.dataset.animate = ''
  const sculptHead = el('header', 'module-head')
  sculptHead.innerHTML = `<h2 class="module-title">Route Network</h2>`
  hoverEl = el('span', 'module-meta sculpt-hover num', 'hover a token or pool')
  sculptHead.appendChild(hoverEl)
  sculptModule.appendChild(sculptHead)
  const canvasWrap = el('div', 'sculpt-canvas')
  canvas = el('canvas')
  canvas.id = 'sculpt'
  canvasWrap.appendChild(canvas)
  sculptModule.appendChild(canvasWrap)
  support.appendChild(sculptModule)

  support.appendChild(renderIntelShell())
  term.appendChild(support)

  sculpture = new Sculpture(canvas, {
    labels: true,
    interactive: true,
    baseParticles: 4,
  })
  sculpture.onHover = onSculptHover
  reveal(term)
  renderTerminalDynamic()
}

function syncSculpture(): void {
  if (!sculpture) return
  const key = state.tokens.map((t) => t.symbol).join(',')
  if (key !== tokenKey) {
    tokenKey = key
    sculpture.setData(
      state.tokens.map((t) => t.symbol),
      state.pools,
    )
  }
  const hot = new Set(
    state.best ? state.best.opp.legs.map((l) => l.pool_idx) : [],
  )
  sculpture.setHot(hot)
}

export function updateTerminal(): void {
  renderFeed()
  terminalHistoryRender?.()
  renderExec()
  renderIntel()
  renderLiquidity()
  syncSculpture()
  tickIntelBaseline()
}

export function onTerminalShown(): void {
  if (canvas) sculpture?.resize()
  renderTerminalDynamic()
}

function renderTerminalDynamic(): void {
  renderFeed()
  terminalHistoryRender?.()
  renderExec()
  renderIntel()
  renderLiquidity()
  syncSculpture()
  tickIntelBaseline()
}
