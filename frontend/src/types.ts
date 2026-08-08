export interface Token {
  symbol: string
  decimals: number
  mint: string | null
}

export interface ScannerSummary {
  base_token: string
  base_amount: number
  min_profit_bps: number
  max_cycle_len: number
  report_min_gross_bps: number
}

export interface StatusResponse {
  mode: string
  tick: number
  uptime_secs: number
  scanner: ScannerSummary
  tokens: Token[]
  pool_count: number
}

export interface Pool {
  token_a: string
  token_b: string
  reserve_a: number
  reserve_b: number
  fee_bps: number
  address: string | null
  dex: string
}

export interface Leg {
  pool_idx: number
  token_in: string
  token_out: string
  amount_in: number
  amount_out: number
}

export interface Opportunity {
  path: string[]
  legs: Leg[]
  start_amount: number
  end_amount: number
  profit_bps: number
  gross_profit_bps: number
  profitable: boolean
}

export interface ScannerEvent {
  tick: number
  prices: [string, number][]
  opportunities: Opportunity[]
  slot?: number
}

export interface HistoricalOpportunity {
  tick: number
  opportunity: Opportunity
  timestamp: number
}

export interface ExecutorResponse {
  capital: number
  starting_capital: number
  trades: number
  wins: number
  losses: number
  roi_bps: number
  total_pnl: number
}
