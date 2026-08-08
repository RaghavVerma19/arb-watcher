import { state } from '../state'

function renderMode(mode: string): void {
  const tag = document.getElementById('mode')
  if (!tag) return
  const map: Record<string, [string, string]> = {
    simulator: ['Simulator', 'mode-simulator'],
    live: ['Live · Jupiter', 'mode-live'],
    onchain: ['Mainnet', 'mode-onchain'],
  }
  const [label, cls] = map[mode] ?? [mode, 'mode-simulator']
  tag.textContent = label
  tag.className = `mode-tag ${cls}`
}

export function setConn(live: boolean): void {
  const label = document.getElementById('conn-label')
  const pill = document.getElementById('conn')
  const dot = document.getElementById('dot')
  if (!label || !pill || !dot) return
  label.textContent = live ? 'Live' : 'Reconnecting…'
  pill.classList.toggle('live', live)
  dot.className = `status-dot ${live ? 'ok' : 'warn'}`
}

export function updateNav(): void {
  if (state.status) renderMode(state.status.mode)
  const rpc = document.getElementById('nav-rpc')
  if (rpc) rpc.textContent = state.rpcLatency ? `${state.rpcLatency}ms` : '—'
}

export function initNav(): void {
  updateNav()
}
