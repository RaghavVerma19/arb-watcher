import { state, modeLabel } from '../state'
import { el, pct } from '../format'

let body: HTMLElement
const vals = new Map<string, HTMLElement>()

function buildRow(key: string, label: string, initial = '—', cls?: string): void {
  const line = el('div', 'exec-row')
  line.appendChild(el('span', 'k', label))
  const v = el('span', `v num${cls ? ' ' + cls : ''}`, initial)
  line.appendChild(v)
  body.appendChild(line)
  vals.set(key, v)
}

export function renderExecShell(): HTMLElement {
  const module = el('aside', 'module module-exec')
  module.dataset.animate = ''
  const head = el('header', 'module-head')
  head.innerHTML = `<h2 class="module-title">Execution</h2>`
  head.appendChild(el('span', 'module-meta', 'readiness'))
  module.appendChild(head)
  body = el('div', 'exec-telemetry')
  module.appendChild(body)

  buildRow('mode', 'Mode')
  buildRow('pools', 'Pools watched')
  buildRow('edge', 'Best edge')

  renderExec()
  return module
}

export function renderExec(): void {
  if (!body) return
  const s = state

  vals.get('mode')!.textContent = modeLabel(s.status?.mode ?? 'simulator')
  vals.get('pools')!.textContent = String(s.pools.length)
  const bestEdge = vals.get('edge')!
  bestEdge.textContent = s.best ? `+${pct(s.best.netBps)}` : '—'
}
