import { reducedMotion } from '../effects'
import type { Pool } from '../types'

interface Node {
  sym: string
  x: number
  y: number
}

interface Edge {
  a: number
  b: number
  poolIdx: number
}

interface Particle {
  edge: number
  t: number
  dir: 1 | -1
  speed: number
  size: number
}

export interface SculptureOptions {
  labels?: boolean
  interactive?: boolean
  ambient?: boolean
  baseParticles?: number
}

export type HoverInfo =
  | { kind: 'node'; sym: string }
  | { kind: 'edge'; poolIdx: number; symA: string; symB: string }
  | null

const JITTER: [number, number][] = [
  [0.04, 0.02],
  [-0.03, 0.05],
  [0.05, -0.04],
  [-0.05, 0.03],
  [0.03, -0.05],
  [0, 0.04],
  [-0.02, -0.03],
]

function distToSegment(px: number, py: number, x1: number, y1: number, x2: number, y2: number): number {
  const dx = x2 - x1
  const dy = y2 - y1
  const len2 = dx * dx + dy * dy
  if (len2 === 0) return Math.hypot(px - x1, py - y1)
  let t = ((px - x1) * dx + (py - y1) * dy) / len2
  t = Math.max(0, Math.min(1, t))
  return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy))
}

export class Sculpture {
  private canvas: HTMLCanvasElement
  private ctx: CanvasRenderingContext2D
  private opts: SculptureOptions
  private tokens: string[] = []
  private pools: Pool[] = []
  private nodes: Node[] = []
  private edges: Edge[] = []
  private particles: Particle[] = []
  private hot = new Set<number>()
  private hotEdge = new Set<number>()
  private raf = 0
  private last = 0
  private spawnT = 0
  private running = false
  private hoveredEdge = -1
  private hoveredNode = -1
  private dpr = 1
  onHover: ((info: HoverInfo) => void) | null = null

  private scale = 1
  private pan = { x: 0, y: 0 }
  private dragging = false
  private dragStart = { x: 0, y: 0 }
  private panStart = { x: 0, y: 0 }

  resize = (): void => {
    this.dpr = Math.min(window.devicePixelRatio || 1, 2)
    const w = this.canvas.clientWidth
    const h = this.canvas.clientHeight
    if (w === 0 || h === 0) return
    this.canvas.width = Math.round(w * this.dpr)
    this.canvas.height = Math.round(h * this.dpr)
    this.ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0)
    this.layout()
    this.seed()
    this.drawOnce()
  }

  private tick = (now: number): void => {
    if (!this.running) return
    this.raf = requestAnimationFrame(this.tick)
    const dt = Math.min((now - this.last) / 16.7, 2.5)
    this.last = now

    if (this.hot.size > 0) {
      this.spawnT += dt
      if (this.spawnT > 14) {
        this.spawnT = 0
        for (const e of this.hotEdge) {
          for (let i = 0; i < 2; i++) this.particles.push(this.makeParticle(e))
        }
      }
    }
    this.particles = this.particles.filter((p) => p.t > 0 && p.t < 1)
    for (const p of this.particles) {
      const speed = p.speed * (this.hotEdge.has(p.edge) ? 2.4 : 1)
      p.t += speed * dt * p.dir
    }
    this.draw()
  }

  private onMove = (e: PointerEvent): void => {
    const r = this.canvas.getBoundingClientRect()
    const x = e.clientX - r.left
    const y = e.clientY - r.top
    let edge = -1
    let node = -1
    let best = 14
    for (let i = 0; i < this.edges.length; i++) {
      const ed = this.edges[i]
      const a = this.nodes[ed.a]
      const b = this.nodes[ed.b]
      const d = distToSegment(x, y, a.x, a.y, b.x, b.y)
      if (d < best) {
        best = d
        edge = i
        node = -1
      }
    }
    for (let i = 0; i < this.nodes.length; i++) {
      const nd = this.nodes[i]
      const d = Math.hypot(x - nd.x, y - nd.y)
      if (d < 16) {
        node = i
        edge = -1
        break
      }
    }
    const changed = this.hoveredEdge !== edge || this.hoveredNode !== node
    this.hoveredEdge = edge
    this.hoveredNode = node
    this.canvas.style.cursor = node >= 0 || edge >= 0 ? 'pointer' : 'default'
    if (changed && this.onHover) {
      if (node >= 0) {
        this.onHover({ kind: 'node', sym: this.nodes[node].sym })
      } else if (edge >= 0) {
        const ed = this.edges[edge]
        this.onHover({
          kind: 'edge',
          poolIdx: ed.poolIdx,
          symA: this.nodes[ed.a].sym,
          symB: this.nodes[ed.b].sym,
        })
      } else {
        this.onHover(null)
      }
    }
  }

  private onLeave = (): void => {
    this.hoveredEdge = -1
    this.hoveredNode = -1
    if (this.onHover) this.onHover(null)
  }

  private onPointerDown = (e: PointerEvent): void => {
    this.dragging = true
    this.dragStart = { x: e.clientX, y: e.clientY }
    this.panStart = { ...this.pan }
    this.canvas.setPointerCapture(e.pointerId)
  }

  private onPointerMove = (e: PointerEvent): void => {
    if (!this.dragging) return
    this.pan.x = this.panStart.x + (e.clientX - this.dragStart.x)
    this.pan.y = this.panStart.y + (e.clientY - this.dragStart.y)
    this.drawOnce()
  }

  private onPointerUp = (): void => {
    this.dragging = false
  }

  private onWheel = (e: WheelEvent): void => {
    e.preventDefault()
    const zoom = e.deltaY < 0 ? 1.08 : 0.92
    const newScale = this.scale * zoom
    if (newScale < 0.4 || newScale > 4) return
    const cx = this.canvas.clientWidth / 2
    const cy = this.canvas.clientHeight / 2
    this.pan.x = cx - (cx - this.pan.x) * zoom
    this.pan.y = cy - (cy - this.pan.y) * zoom
    this.scale = newScale
    this.drawOnce()
  }

  private onDblClick = (): void => {
    this.scale = 1
    this.pan = { x: 0, y: 0 }
    this.drawOnce()
  }

  constructor(canvas: HTMLCanvasElement, opts: SculptureOptions = {}) {
    this.canvas = canvas
    this.ctx = canvas.getContext('2d')!
    this.opts = opts

    this.resize()
    window.addEventListener('resize', this.resize)
    if (opts.interactive) {
      canvas.addEventListener('pointermove', this.onMove)
      canvas.addEventListener('pointerleave', this.onLeave)
      canvas.addEventListener('pointerdown', this.onPointerDown)
      window.addEventListener('pointermove', this.onPointerMove)
      window.addEventListener('pointerup', this.onPointerUp)
      canvas.addEventListener('wheel', this.onWheel, { passive: false })
      canvas.addEventListener('dblclick', this.onDblClick)
    }
    if (!reducedMotion) {
      this.running = true
      this.last = performance.now()
      this.raf = requestAnimationFrame(this.tick)
    }
  }

  setData(tokens: string[], pools: Pool[]): void {
    this.tokens = tokens
    this.pools = pools
    this.scale = 1
    this.pan = { x: 0, y: 0 }
    this.layout()
    this.seed()
    this.drawOnce()
  }

  setHot(pools: Set<number>): void {
    this.hot = pools
    this.hotEdge.clear()
    this.edges.forEach((e, i) => {
      if (this.hot.has(e.poolIdx)) this.hotEdge.add(i)
    })
  }

  private layout(): void {
    const w = this.canvas.width
    const h = this.canvas.height
    const n = this.tokens.length
    const rx = Math.min(w * (this.opts.ambient ? 0.44 : 0.4), 460)
    const ry = Math.min(h * (this.opts.ambient ? 0.36 : 0.3), 240)
    const cx = w / 2
    const cy = h / 2
    this.nodes = this.tokens.map((sym, i) => {
      const a = (i / Math.max(n, 1)) * Math.PI * 2 - Math.PI / 2
      const [jx, jy] = JITTER[i % JITTER.length]
      return {
        sym,
        x: cx + Math.cos(a) * rx * (1 + jx),
        y: cy + Math.sin(a) * ry * (1 + jy),
      }
    })
    const idx = new Map<string, number>()
    this.nodes.forEach((nd, i) => idx.set(nd.sym, i))
    const seen = new Set<string>()
    this.edges = []
    for (const p of this.pools) {
      const ia = idx.get(p.token_a)
      const ib = idx.get(p.token_b)
      if (ia === undefined || ib === undefined) continue
      const key = [Math.min(ia, ib), Math.max(ia, ib)].join(':')
      if (seen.has(key)) continue
      seen.add(key)
      this.edges.push({ a: ia, b: ib, poolIdx: this.pools.indexOf(p) })
    }
  }

  private seed(): void {
    const base = this.opts.baseParticles ?? 3
    this.particles = []
    for (let e = 0; e < this.edges.length; e++) {
      for (let i = 0; i < base; i++) {
        this.particles.push(this.makeParticle(e))
      }
    }
  }

  private makeParticle = (edge: number): Particle => {
    return {
      edge,
      t: Math.random(),
      dir: Math.random() < 0.5 ? 1 : -1,
      speed: 0.0008 + Math.random() * 0.0014,
      size: 0.8 + Math.random() * 1.2,
    }
  }

  private drawOnce = (): void => {
    if (!reducedMotion) return
    this.draw()
  }

  private draw = (): void => {
    const ctx = this.ctx
    const w = this.canvas.clientWidth
    const h = this.canvas.clientHeight
    const dim = this.opts.ambient ? 0.55 : 1
    ctx.clearRect(0, 0, w, h)

    ctx.save()
    ctx.translate(this.pan.x, this.pan.y)
    ctx.scale(this.scale, this.scale)

    for (let i = 0; i < this.edges.length; i++) {
      const ed = this.edges[i]
      const a = this.nodes[ed.a]
      const b = this.nodes[ed.b]
      const hot = this.hotEdge.has(i)
      const hovered = this.hoveredEdge === i
      ctx.beginPath()
      ctx.moveTo(a.x, a.y)
      ctx.lineTo(b.x, b.y)
      ctx.strokeStyle = hot
        ? `rgba(62,207,142,${0.4 * dim})`
        : hovered
          ? `rgba(201,191,169,${0.5 * dim})`
          : `rgba(157,147,127,${0.16 * dim})`
      ctx.lineWidth = hot ? 1.5 : hovered ? 1.4 : 1
      ctx.stroke()
    }

    for (const p of this.particles) {
      const ed = this.edges[p.edge]
      if (!ed) continue
      const a = this.nodes[ed.a]
      const b = this.nodes[ed.b]
      const x = a.x + (b.x - a.x) * p.t
      const y = a.y + (b.y - a.y) * p.t
      const hot = this.hotEdge.has(p.edge)
      ctx.beginPath()
      ctx.arc(x, y, p.size, 0, Math.PI * 2)
      ctx.fillStyle = hot
        ? `rgba(62,207,142,${0.75 * dim})`
        : `rgba(201,191,169,${0.3 * dim})`
      ctx.fill()
    }

    for (let i = 0; i < this.nodes.length; i++) {
      const nd = this.nodes[i]
      const inHot = this.nodes.some((n, j) => n.sym === nd.sym && this.isHotNode(j))
      const hovered = this.hoveredNode === i
      const hotNode = inHot || hovered
      ctx.beginPath()
      ctx.arc(nd.x, nd.y, hotNode ? 5 : 4, 0, Math.PI * 2)
      ctx.fillStyle = '#1d1912'
      ctx.fill()
      ctx.strokeStyle = hotNode
        ? `rgba(62,207,142,${0.9 * dim})`
        : `rgba(248,242,231,${0.22 * dim})`
      ctx.lineWidth = hotNode ? 1.6 : 1
      ctx.stroke()
      if (this.opts.labels) {
        ctx.font = `600 ${hotNode ? 11 : 10}px Manrope, Inter, system-ui, sans-serif`
        ctx.fillStyle = hotNode
          ? `rgba(62,207,142,${0.95 * dim})`
          : `rgba(201,191,169,${0.75 * dim})`
        ctx.textAlign = 'center'
        ctx.fillText(nd.sym, nd.x, nd.y + (hotNode ? 16 : 15))
      }
    }

    ctx.restore()
  }

  private isHotNode = (nodeIdx: number): boolean => {
    for (const e of this.hotEdge) {
      const ed = this.edges[e]
      if (ed.a === nodeIdx || ed.b === nodeIdx) return true
    }
    return false
  }

  destroy(): void {
    this.running = false
    cancelAnimationFrame(this.raf)
    window.removeEventListener('resize', this.resize)
    this.canvas.removeEventListener('pointermove', this.onMove)
    this.canvas.removeEventListener('pointerleave', this.onLeave)
    this.canvas.removeEventListener('pointerdown', this.onPointerDown)
    window.removeEventListener('pointermove', this.onPointerMove)
    window.removeEventListener('pointerup', this.onPointerUp)
    this.canvas.removeEventListener('wheel', this.onWheel)
    this.canvas.removeEventListener('dblclick', this.onDblClick)
  }
}
