import './style.css'

import { boot, subscribe, onConnChange, state, modeLabel } from './state'
import { el } from './format'
import { initBg, updateBg } from './views/bg'
import { initNav, updateNav, setConn } from './views/nav'
import { renderLanding, updateLanding } from './views/landing'
import { renderHistoryShell } from './views/history'
import { renderTerminal, updateTerminal, onTerminalShown } from './terminal'
import { initRouter, onViewShown } from './router'
import { isView } from './viewState'

function initModal(): void {
  const modal = document.getElementById('modal') as HTMLElement
  const body = document.getElementById('modal-body') as HTMLElement
  const closeBtn = document.getElementById('modal-close') as HTMLElement

  const open = (): void => {
    renderModalBody(body)
    modal.hidden = false
    document.body.classList.add('modal-open')
    closeBtn.focus()
  }
  const close = (): void => {
    modal.hidden = true
    document.body.classList.remove('modal-open')
  }

  document.querySelectorAll('[data-modal]').forEach((b) => {
    b.addEventListener('click', open)
  })
  closeBtn.addEventListener('click', close)
  modal.querySelector('.modal-backdrop')?.addEventListener('click', close)
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !modal.hidden) close()
  })
}

function renderModalBody(body: HTMLElement): void {
  body.replaceChildren()
  const wrap = el('div')
  const r = (k: string, v: string): void => {
    const line = el('div', 'exec-row')
    line.appendChild(el('span', 'k', k))
    line.appendChild(el('span', 'v num', v))
    wrap.appendChild(line)
  }

  const s = state
  const uptime = s.status
    ? `${Math.floor(s.status.uptime_secs / 60)}m ${s.status.uptime_secs % 60}s`
    : '—'
  r('Mode', modeLabel(s.status?.mode ?? 'simulator'))
  r('Uptime', uptime)
  r('Pools watched', String((s.pools.length || s.status?.pool_count) ?? 0))
  r('Tokens', String(s.tokens.length))
  r('Base token', `${s.baseToken} · ${fmtAmount(s.baseAmount, s.baseDec)} scan`)
  r('Min profit', `${s.minProfitBps} bp`)
  r('Tick', String(s.latestEvent?.tick ?? 0))
  r('RPC', s.rpcLatency ? `${s.rpcLatency}ms` : '—')
  r('Feed', `${Date.now() - s.lastEventAt}ms`)

  body.appendChild(wrap)
  body.appendChild(
    el(
      'p',
      'explainer',
      'All execution is paper-traded against the detected price with zero latency — a learning vehicle, not a money printer. Live swaps stay behind a kill switch.',
    ),
  )
}

function fmtAmount(units: number, dec: number): string {
  return (units / 10 ** dec).toLocaleString(undefined, {
    maximumFractionDigits: dec,
  })
}

function init(): void {
  const bg = initBg()

  renderLanding(bg)
  const historyShell = renderHistoryShell()
  const historyHost = document.getElementById('view-history') as HTMLElement
  historyHost.appendChild(historyShell.module)
  renderTerminal()
  initNav()
  initRouter()
  initModal()

  onViewShown(() => {
    if (isView('terminal')) onTerminalShown()
    if (isView('history')) historyShell.render()
  })
  onConnChange(setConn)

  subscribe(() => {
    updateNav()
    updateLanding(bg)
    historyShell.render()
    updateTerminal()
  })

  void boot().then(() => updateBg(bg))
}

init()
