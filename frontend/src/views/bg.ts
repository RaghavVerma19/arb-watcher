import { Sculpture } from './sculpture'
import { state } from '../state'

let tokenKey = ''

export function initBg(): Sculpture {
  const canvas = document.getElementById('bg') as HTMLCanvasElement
  return new Sculpture(canvas, {
    ambient: true,
    labels: false,
    baseParticles: 2,
  })
}

export function updateBg(s: Sculpture): void {
  const key = state.tokens.map((t) => t.symbol).join(',')
  if (key !== tokenKey) {
    tokenKey = key
    s.setData(
      state.tokens.map((t) => t.symbol),
      state.pools,
    )
  }
  const hot = new Set(
    state.best ? state.best.opp.legs.map((l) => l.pool_idx) : [],
  )
  s.setHot(hot)
}
