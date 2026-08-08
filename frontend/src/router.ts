import { fadeIn, fadeOut, fadeVeil, reveal, reducedMotion } from './effects'
import { setView, currentView, type ViewId } from './viewState'

const landingEl = document.getElementById('view-landing') as HTMLElement
const historyEl = document.getElementById('view-history') as HTMLElement
const terminalEl = document.getElementById('view-terminal') as HTMLElement
const veil = document.getElementById('veil') as HTMLElement
const nav = document.getElementById('nav') as HTMLElement

function setNavStyle(view: ViewId): void {
  document.body.classList.toggle('in-terminal', view === 'terminal')
  nav.classList.toggle('glass', view === 'terminal')
}

function syncActiveLinks(view: ViewId): void {
  document.querySelectorAll<HTMLButtonElement>('.nav-link[data-view]').forEach((b) => {
    b.classList.toggle('is-active', b.dataset.view === view)
  })
}

let shownHandler: (() => void) | null = null
export function onViewShown(fn: () => void): void {
  shownHandler = fn
}

export async function go(view: ViewId): Promise<void> {
  if (view === currentView) return

  const fromEl = (() => {
    switch (currentView) {
      case 'landing': return landingEl
      case 'history': return historyEl
      case 'terminal': return terminalEl
    }
  })()

  const toEl = (() => {
    switch (view) {
      case 'landing': return landingEl
      case 'history': return historyEl
      case 'terminal': return terminalEl
    }
  })()

  if (reducedMotion) {
    fromEl.hidden = true
    toEl.hidden = false
    setView(view)
    setNavStyle(view)
    syncActiveLinks(view)
    window.scrollTo(0, 0)
    reveal(toEl)
    shownHandler?.()
    return
  }

  await fadeOut(fromEl, 420)
  fadeVeil(veil, true, 180)
  fromEl.hidden = true
  toEl.hidden = false
  setView(view)
  setNavStyle(view)
  syncActiveLinks(view)
  window.scrollTo(0, 0)
  reveal(toEl)
  fadeVeil(veil, false, 260)
  fadeIn(toEl, 700)
  shownHandler?.()
}

export function onTerminalEnter(): void {
  if (currentView === 'terminal') return
  void go('terminal')
}

export function initRouter(): void {
  landingEl.addEventListener('click', (e) => {
    const target = (e.target as HTMLElement).closest('[data-enter]')
    if (target) onTerminalEnter()
  })

  document.querySelectorAll<HTMLButtonElement>('.nav-link[data-view]').forEach((b) => {
    b.addEventListener('click', () => {
      const v = (b.dataset.view as ViewId) ?? 'landing'
      if (v === currentView) return
      void go(v)
    })
  })

  const brand = document.getElementById('nav-brand') as HTMLButtonElement
  brand.addEventListener('click', () => {
    if (currentView !== 'landing') void go('landing')
  })

  syncActiveLinks('landing')
  setNavStyle('landing')
}
