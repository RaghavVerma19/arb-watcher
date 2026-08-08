export type ViewId = 'landing' | 'history' | 'terminal'

export let currentView: ViewId = 'landing'

export function setView(v: ViewId): void {
  currentView = v
}

export function isView(v: ViewId): boolean {
  return currentView === v
}
