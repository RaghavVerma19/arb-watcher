import { state } from '../state'
import { el, pct } from '../format'

export function renderHistoryShell(): { module: HTMLElement; render: () => void } {
  const module = el('section', 'module module-history')
  const head = el('header', 'module-head')
  head.innerHTML = `<h2 class="module-title">Edge History</h2>`
  const meta = el('span', 'module-meta', '—')
  head.appendChild(meta)
  module.appendChild(head)

  const chartWrap = el('div', 'history-chart')
  const chartCanvas = el('canvas')
  chartCanvas.id = 'history-chart'
  chartWrap.appendChild(chartCanvas)
  module.appendChild(chartWrap)

  const body = el('div', 'history-body')
  module.appendChild(body)

  function drawChart(): void {
    if (!chartCanvas) return
    const ctx = chartCanvas.getContext('2d')
    if (!ctx) return

    const history = state.historicalOpportunities
    if (history.length === 0) return

    const rect = chartCanvas.getBoundingClientRect()
    const dpr = window.devicePixelRatio || 1
    chartCanvas.width = rect.width * dpr
    chartCanvas.height = 180 * dpr
    ctx.scale(dpr, dpr)

    const width = rect.width
    const height = 180
    const pad = { top: 20, right: 20, bottom: 30, left: 60 }
    const plotW = width - pad.left - pad.right
    const plotH = height - pad.top - pad.bottom

    const items = history.slice(-120)
    const bpsValues = items.map((h) => h.opportunity.profit_bps)
    const maxBps = Math.max(...bpsValues.map(Math.abs), 100)

    ctx.clearRect(0, 0, width, height)

    ctx.strokeStyle = 'rgba(255,255,255,0.06)'
    ctx.lineWidth = 1
    for (let i = 0; i <= 4; i++) {
      const y = pad.top + (plotH / 4) * i
      ctx.beginPath()
      ctx.moveTo(pad.left, y)
      ctx.lineTo(width - pad.right, y)
      ctx.stroke()
    }

    const barW = plotW / items.length
    items.forEach((h, i) => {
      const x = pad.left + i * barW
      const bps = h.opportunity.profit_bps
      const barH = (Math.abs(bps) / maxBps) * plotH
      const y = bps >= 0 ? pad.top + plotH - barH : pad.top + plotH

      ctx.fillStyle = h.opportunity.profitable ? 'rgba(160,180,190,0.8)' : 'rgba(255,255,255,0.12)'
      ctx.fillRect(x, y, Math.max(barW - 1, 1), barH)
    })

    ctx.fillStyle = 'rgba(255,255,255,0.35)'
    ctx.font = '11px IBM Plex Mono, monospace'
    ctx.textAlign = 'right'
    ctx.fillText(`+${maxBps} bp`, pad.left - 8, pad.top + 10)
    ctx.fillText('0 bp', pad.left - 8, pad.top + plotH)
    ctx.fillText(`-${maxBps} bp`, pad.left - 8, pad.top + plotH + 10)
  }

  function render(): void {
    body.replaceChildren()

    const history = state.historicalOpportunities
    if (history.length === 0) {
      meta.textContent = '—'
      body.appendChild(el('p', 'empty-state', 'No opportunities recorded yet.'))
      return
    }

    meta.textContent = `${history.length} edges`

    const sorted = [...history].sort((a, b) => b.timestamp - a.timestamp)

    const table = el('table', 'history-table')
    const thead = el('thead')
    const headerRow = el('tr')

    const headers = ['Time', 'Tick', 'Path', 'Profit', 'Status']
    headers.forEach((h) => {
      const th = el('th', '', h)
      headerRow.appendChild(th)
    })
    thead.appendChild(headerRow)
    table.appendChild(thead)

    const tbody = el('tbody')
    sorted.forEach((item) => {
      const row = el('tr')
      const opp = item.opportunity

      const timeCell = el('td', '')
      const date = new Date(item.timestamp * 1000)
      timeCell.textContent = date.toLocaleTimeString()
      row.appendChild(timeCell)

      const tickCell = el('td', 'num', String(item.tick))
      row.appendChild(tickCell)

      const pathCell = el('td', '')
      const pathSpan = el('span', 'path', opp.path.join(' → '))
      pathCell.appendChild(pathSpan)
      row.appendChild(pathCell)

      const profitCell = el('td', `num ${opp.profitable ? 'profit' : 'loss'}`)
      profitCell.textContent = `${pct(opp.profit_bps)} (${opp.profitable ? '+' : ''}${opp.profit_bps} bp)`
      row.appendChild(profitCell)

      const statusCell = el('td', '')
      const badge = el('span', `badge ${opp.profitable ? 'profit' : 'near-miss'}`)
      badge.textContent = opp.profitable ? 'Profitable' : 'Near-miss'
      statusCell.appendChild(badge)
      row.appendChild(statusCell)

      tbody.appendChild(row)
    })

    table.appendChild(tbody)
    body.appendChild(table)

    requestAnimationFrame(() => drawChart())
  }

  render()
  return { module, render }
}
