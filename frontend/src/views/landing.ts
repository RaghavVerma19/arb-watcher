import { state } from '../state'
import { countNumber, reveal } from '../effects'
import { el } from '../format'
import { renderRouteBadges } from '../analysis'
import type { Sculpture } from './sculpture'

let heroBps = 0
let metricNum: HTMLElement
let metricLabel: HTMLElement
let metricDetail: HTMLElement
let routeRow: HTMLElement
let captionEl: HTMLElement

export function renderLanding(bg: Sculpture): void {
  const host = document.getElementById('view-landing') as HTMLElement
  host.replaceChildren()

  const hero = el('section', 'hero')
  hero.dataset.sculpture = 'bg'

  const text = el('div', 'hero-text')

  const eyebrow = el('p', 'hero-eyebrow', 'Solana · Triangular Arbitrage Intelligence')
  eyebrow.dataset.animate = ''

  const title = el('h1', 'hero-title')
  title.dataset.animate = ''
  title.innerHTML = 'Real-Time Arbitrage<br /><span class="hero-title-dim">Intelligence</span>'

  const sub = el('p', 'hero-sub', 'Finding market inefficiencies before everyone else.')
  sub.dataset.animate = ''

  const metric = el('div', 'hero-metric')
  metric.dataset.animate = ''
  const numWrap = el('div', 'hero-metric-num')
  metricNum = el('span', 'num', '—')
  numWrap.appendChild(metricNum)
  metric.appendChild(numWrap)
  const labelWrap = el('div', 'hero-metric-copy')
  metricLabel = el('p', 'hero-metric-label', 'Watching — no edge right now')
  metricDetail = el('p', 'hero-metric-detail', '')
  routeRow = el('div', 'hero-route')
  labelWrap.append(metricLabel, metricDetail, routeRow)
  metric.appendChild(labelWrap)

  const actions = el('div', 'hero-actions')
  actions.dataset.animate = ''
  const cta = el('button', 'btn btn-primary btn-lg', 'Enter Terminal')
  cta.dataset.enter = ''
  cta.setAttribute('type', 'button')
  actions.appendChild(cta)
  actions.appendChild(
    el('span', 'hero-hint', 'live scanner · paper trading only'),
  )

  text.append(eyebrow, title, sub, metric, actions)
  hero.appendChild(text)

  captionEl = el('p', 'hero-caption', 'watching pools…')
  captionEl.dataset.animate = ''
  hero.appendChild(captionEl)

  host.appendChild(hero)
  reveal(host)
  updateLanding(bg)
}

export function updateLanding(bg: Sculpture): void {
  const best = state.best
  const tokenCount = state.tokens.length
  const poolCount = state.pools.length || state.status?.pool_count

  captionEl.textContent = `watching ${poolCount} ${poolCount === 1 ? 'pool' : 'pools'} · ${tokenCount} ${tokenCount === 1 ? 'token' : 'tokens'} · ${state.status?.mode ?? 'simulator'}`

  if (best) {
    const target = best.netBps
    if (target !== heroBps) {
      countNumber(
        metricNum,
        heroBps,
        target,
        (v) => `${v >= 0 ? '+' : ''}${(v / 100).toFixed(2)}%`,
        600,
      )
      heroBps = target
    }
    metricLabel.textContent = 'Best Opportunity Right Now'
    metricDetail.textContent = `Detected across ${new Set(best.opp.legs.map((l) => l.pool_idx)).size} of ${poolCount} liquidity pools.`
    routeRow.replaceChildren(renderRouteBadges(best.opp.path))
  } else {
    metricNum.textContent = '—'
    heroBps = 0
    metricLabel.textContent = 'Watching — no edge right now'
    metricDetail.textContent = `Scanning ${poolCount} pools across ${tokenCount} tokens for the first divergence.`
    routeRow.replaceChildren()
  }

  if (best) {
    bg.setHot(new Set(best.opp.legs.map((l) => l.pool_idx)))
  } else {
    bg.setHot(new Set())
  }
}
