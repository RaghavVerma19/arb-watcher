import type {
  HistoricalOpportunity,
  Opportunity,
  Pool,
  ScannerEvent,
  StatusResponse,
  Token,
  ExecutorResponse,
} from './types'
import { analyze, type Analysis } from './analysis'

export interface VanishItem {
  key: string
  bps: number
  ts: number
}

export const filters = {
  minRoiBps: 0,
  minLiqUsd: 0,
  confMin: 0,
  poolOn: new Set<number>(),
  showNearMiss: true,
}

export const state = {
  status: null as StatusResponse | null,
  pools: [] as Pool[],
  tokens: [] as Token[],
  decimals: new Map<string, number>(),
  baseToken: 'USDC',
  baseDec: 6,
  baseAmount: 0,
  minProfitBps: 10,
  reportMinGrossBps: 5,
  latestEvent: null as ScannerEvent | null,
  lastPrices: new Map<string, number>(),
  priceHistory: new Map<string, number[]>(),
  analyses: new Map<string, Analysis>(),
  best: null as Analysis | null,
  prevBpsByRoute: new Map<string, number>(),
  oppSeen: new Map<string, number>(),
  vanished: [] as VanishItem[],
  history: [] as { tick: number; maxBps: number; count: number }[],
  historicalOpportunities: [] as HistoricalOpportunity[],
  expanded: new Set<string>(),
  lastEventAt: Date.now(),
  rpcLatency: 0,
  executor: null as ExecutorResponse | null,
}

type DataListener = () => void
const listeners = new Set<DataListener>()

export function subscribe(fn: DataListener): void {
  listeners.add(fn)
}

function notify(): void {
  for (const fn of listeners) fn()
}

function processEvent(ev: ScannerEvent): void {
  state.latestEvent = ev
  state.lastEventAt = Date.now()

  if (ev.prices.length > 0) {
    state.lastPrices = new Map(ev.prices)
    for (const [sym, price] of ev.prices) {
      const arr = state.priceHistory.get(sym) ?? []
      arr.push(price)
      if (arr.length > 60) arr.shift()
      state.priceHistory.set(sym, arr)
    }
  }

  const seenNow = new Set<string>()
  for (const opp of ev.opportunities) {
    const key = opp.path.join('→')
    seenNow.add(key)
    state.oppSeen.set(key, (state.oppSeen.get(key) ?? 0) + 1)
  }
  for (const [key, bps] of state.prevBpsByRoute) {
    if (!seenNow.has(key)) state.vanished.push({ key, bps, ts: Date.now() })
  }
  state.vanished = state.vanished.filter((v) => Date.now() - v.ts < 3500)

  state.analyses.clear()
  for (const opp of ev.opportunities) {
    state.analyses.set(opp.path.join('→'), analyze(opp, ctx()))
  }
  state.best =
    [...state.analyses.values()].sort(
      (a, b) =>
        Number(b.opp.profitable) - Number(a.opp.profitable) ||
        b.netBps - a.netBps,
    )[0] ?? null

  state.history.push({
    tick: ev.tick,
    maxBps: ev.opportunities
      .filter((o) => o.profitable)
      .reduce((m, o) => Math.max(m, o.profit_bps), 0),
    count: ev.opportunities.length,
  })
  if (state.history.length > 120) state.history.shift()

  state.prevBpsByRoute = new Map(
    ev.opportunities.map((o) => [o.path.join('→'), o.profit_bps]),
  )
}

function ctx() {
  return {
    pools: state.pools,
    decimals: state.decimals,
    baseToken: state.baseToken,
    baseDec: state.baseDec,
    minProfitBps: state.minProfitBps,
    lastPrices: state.lastPrices,
    oppSeen: state.oppSeen,
  }
}

export function modeLabel(mode: string): string {
  if (mode === 'onchain') return 'Mainnet'
  if (mode === 'live') return 'Jupiter Live'
  return 'Simulator'
}

async function loadStatus(): Promise<void> {
  const res = await fetch('/api/status')
  const s: StatusResponse = await res.json()
  state.status = s
  state.tokens = s.tokens
  state.decimals = new Map(s.tokens.map((t) => [t.symbol, t.decimals]))
  state.baseToken = s.scanner.base_token
  state.baseDec = state.decimals.get(state.baseToken) ?? 6
  state.baseAmount = s.scanner.base_amount
  state.minProfitBps = s.scanner.min_profit_bps
  state.reportMinGrossBps = s.scanner.report_min_gross_bps
  for (let i = 0; i < s.pool_count; i++) filters.poolOn.add(i)
}

async function loadPools(): Promise<void> {
  const res = await fetch('/api/pools')
  state.pools = await res.json()
}

async function loadOpps(): Promise<void> {
  const res = await fetch('/api/opportunities')
  const opps: Opportunity[] = await res.json()
  const ev: ScannerEvent = {
    tick: state.status?.tick ?? 0,
    prices: [],
    opportunities: opps,
  }
  processEvent(ev)
}

const MAX_HISTORY_AGE_MS = 30 * 60 * 1000 // 30 minutes
const MAX_HISTORY_COUNT = 500

function trimHistory(): void {
  const now = Date.now()
  state.historicalOpportunities = state.historicalOpportunities
    .filter((h) => now - h.timestamp * 1000 < MAX_HISTORY_AGE_MS)
    .slice(-MAX_HISTORY_COUNT)
}

async function loadHistory(): Promise<void> {
  const res = await fetch('/api/history')
  const history: HistoricalOpportunity[] = await res.json()
  state.historicalOpportunities = history
  trimHistory()
}

async function loadExecutor(): Promise<void> {
  try {
    const res = await fetch('/api/executor')
    if (res.ok) {
      state.executor = await res.json()
    }
  } catch {
    /* keep null */
  }
}

async function pollRpcLatency(): Promise<void> {
  const t0 = performance.now()
  try {
    await fetch('/api/status', { cache: 'no-store' })
    state.rpcLatency = Math.round(performance.now() - t0)
  } catch {
    /* keep last value */
  }
}

function connectWs(): void {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  const wsHost = (import.meta.env.VITE_WS_HOST ?? '').trim() || location.host
  const ws = new WebSocket(`${proto}://${wsHost}/ws`)
  ws.onopen = () => setConn(true)
  ws.onmessage = (ev: MessageEvent<string>) => {
    const event: ScannerEvent = JSON.parse(ev.data)
    processEvent(event)
    notify()
  }
  ws.onclose = () => {
    setConn(false)
    setTimeout(connectWs, 1000)
  }
  ws.onerror = () => ws.close()
}

let connHandler: ((live: boolean) => void) | null = null
export function onConnChange(fn: (live: boolean) => void): void {
  connHandler = fn
}
function setConn(live: boolean): void {
  connHandler?.(live)
}

export async function boot(): Promise<void> {
  await Promise.all([loadStatus(), loadPools()])
  await Promise.all([loadOpps(), loadHistory(), loadExecutor()])
  notify()
  connectWs()
  setInterval(pollRpcLatency, 10000)
  setInterval(trimHistory, 60000)
}
