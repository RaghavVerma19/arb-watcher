import type { Opportunity, Pool } from './types'
import { clamp, fmtBig, pct, el } from './format'

export interface LegInfo {
  pool: Pool
  pool_idx: number
  token_in: string
  token_out: string
  amount_in: number
  amount_out: number
  fee_bps: number
  effRate: number
  midRate: number
  impactPct: number
  driver: boolean
}

export interface Analysis {
  key: string
  opp: Opportunity
  legs: LegInfo[]
  grossBps: number
  feeBps: number
  impactBps: number
  netBps: number
  netPct: number
  impactPctTotal: number
  maxSizeRaw: bigint
  maxSizeDisplay: string
  maxSizeUsd: number
  confidence: number
  explain: string
}

export interface AnalysisContext {
  pools: Pool[]
  decimals: Map<string, number>
  baseToken: string
  baseDec: number
  minProfitBps: number
  lastPrices: Map<string, number>
  oppSeen: Map<string, number>
}

export function reserveOf(p: Pool, sym: string): number {
  return p.token_a === sym ? p.reserve_a : p.reserve_b
}

export function poolName(p: Pool): string {
  return `${p.token_a}/${p.token_b}`
}

export function chainEnd(
  startRaw: bigint,
  opp: Opportunity,
  pools: Pool[],
): bigint {
  let amt = startRaw
  for (const leg of opp.legs) {
    const p = pools[leg.pool_idx]
    if (!p) return 0n
    const resIn = BigInt(reserveOf(p, leg.token_in))
    const resOut = BigInt(reserveOf(p, leg.token_out))
    const f = BigInt(10000 - p.fee_bps)
    const inAdj = (amt * f) / 10000n
    amt = (resOut * inAdj) / (resIn + inAdj)
  }
  return amt
}

function maxProfitableSize(opp: Opportunity, pools: Pool[]): bigint {
  let lo = 1n
  let hi = 10n ** 15n
  let bestSize = 1n
  while (lo <= hi) {
    const mid = (lo + hi) / 2n
    if (chainEnd(mid, opp, pools) >= mid) {
      bestSize = mid
      lo = mid + 1n
    } else {
      hi = mid - 1n
    }
  }
  return bestSize
}

export function analyze(opp: Opportunity, ctx: AnalysisContext): Analysis {
  const { pools, decimals, baseToken, baseDec, minProfitBps, lastPrices, oppSeen } = ctx

  const legs: LegInfo[] = opp.legs.map((leg) => {
    const pool = pools[leg.pool_idx]
    const resIn = reserveOf(pool, leg.token_in)
    const resOut = reserveOf(pool, leg.token_out)
    const midRate = resIn > 0 ? resOut / resIn : 0
    const effRate = leg.amount_in > 0 ? leg.amount_out / leg.amount_in : 0
    const fee = pool.fee_bps / 10000
    const impactPct =
      midRate > 0 && effRate > 0
        ? (1 - effRate / (midRate * (1 - fee))) * 100
        : 0
    return {
      pool,
      pool_idx: leg.pool_idx,
      token_in: leg.token_in,
      token_out: leg.token_out,
      amount_in: leg.amount_in,
      amount_out: leg.amount_out,
      fee_bps: pool.fee_bps,
      effRate,
      midRate,
      impactPct,
      driver: false,
    }
  })

  let grossRatio = 1
  let feeRatio = 1
  for (const leg of legs) {
    grossRatio *= leg.midRate
    feeRatio *= 1 - leg.fee_bps / 10000
  }
  const netRatio = opp.end_amount / opp.start_amount
  const dragRatio = grossRatio / netRatio
  const impactRatio = dragRatio / feeRatio

  let driverIdx = 0
  let driverEdge = -Infinity
  legs.forEach((leg, i) => {
    const edge = leg.midRate > 0 ? leg.effRate / leg.midRate - 1 : -1
    if (edge > driverEdge) {
      driverEdge = edge
      driverIdx = i
    }
  })
  legs[driverIdx].driver = true

  const grossBps = (grossRatio - 1) * 10000
  const feeBps = (1 - feeRatio) * 10000
  const impactBps = (1 - impactRatio) * 10000
  const netBps = opp.profit_bps
  const impactPctTotal = (1 - impactRatio) * 100

  const maxSizeRaw = maxProfitableSize(opp, pools)
  const maxSizeDisplay = fmtBig(maxSizeRaw, baseDec)
  const priceOfBase = lastPrices.get(baseToken) ?? 1
  const maxSizeUsd =
    maxSizeRaw < 10n ** 15n
      ? (Number(maxSizeRaw) / 10 ** baseDec) * priceOfBase
      : Infinity

  const marginScore = clamp(netBps / Math.max(minProfitBps * 2, 1) / 1.5, 0, 1)
  const depthUsd = Math.min(
    ...legs.map((leg) => {
      const pa = lastPrices.get(leg.pool.token_a) ?? 0
      const pb = lastPrices.get(leg.pool.token_b) ?? 0
      const da = decimals.get(leg.pool.token_a) ?? 0
      const db = decimals.get(leg.pool.token_b) ?? 0
      return Math.min(
        (leg.pool.reserve_a / 10 ** da) * pa,
        (leg.pool.reserve_b / 10 ** db) * pb,
      )
    }),
  )
  const depthScore = clamp(Math.log10(Math.max(depthUsd, 1000)) / 6, 0, 1)
  const persist = oppSeen.get(opp.path.join('→')) ?? 0
  const persistScore = clamp(persist / 12, 0, 1)
  const confidence = Math.round(
    (0.5 * marginScore + 0.3 * depthScore + 0.2 * persistScore) * 100,
  )

  const d = legs[driverIdx]
  const dPct = ((d.effRate / d.midRate - 1) * 100).toFixed(2)
  const explain =
    `Pool #${d.pool_idx} (${poolName(d.pool)}) is pricing the ` +
    `${d.token_in}→${d.token_out} leg at ${dPct}% against its own midpoint ` +
    `— the imbalance that opens this cycle. Net ${pct(netBps)} after ` +
    `${feeBps.toFixed(0)} bp in pool fees and ~${impactPctTotal.toFixed(2)}% ` +
    `price impact. The edge survives up to ~${maxSizeDisplay} ` +
    `${baseToken}; beyond that, impact erodes it.`

  return {
    key: opp.path.join('→'),
    opp,
    legs,
    grossBps,
    feeBps,
    impactBps,
    netBps,
    netPct: netBps / 100,
    impactPctTotal,
    maxSizeRaw,
    maxSizeDisplay,
    maxSizeUsd,
    confidence,
    explain,
  }
}

export function renderRouteBadges(path: string[]): HTMLElement {
  const rl = el('div', 'route-line')
  path.forEach((sym, i) => {
    if (i > 0) rl.appendChild(el('span', 'sep', '→'))
    rl.appendChild(el('span', 'token-badge', sym))
  })
  return rl
}
