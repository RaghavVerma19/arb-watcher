import { animate, stagger } from 'motion'

const mqReduced = window.matchMedia('(prefers-reduced-motion: reduce)')
export let reducedMotion = mqReduced.matches
mqReduced.addEventListener('change', (e) => {
  reducedMotion = e.matches
})

const ease = [0.2, 0.8, 0.2, 1] as const

/** Animate a number into a DOM text node, counting up or down. */
export function countNumber(
  node: HTMLElement,
  from: number,
  to: number,
  format: (v: number) => string,
  duration = 500,
): void {
  if (reducedMotion) {
    node.textContent = format(to)
    return
  }
  animate(from, to, {
    duration: duration / 1000,
    ease,
    onUpdate: (v) => {
      node.textContent = format(v)
    },
  })
}

/** Reveal a set of [data-animate] children with a cinematic stagger. */
export function reveal(target: Element | null): void {
  if (!target) return
  const items = target.querySelectorAll<HTMLElement>('[data-animate]')
  if (reducedMotion || items.length === 0) return
  animate(
    items,
    { opacity: 1, filter: 'blur(0px)', transform: 'translateY(0px)' },
    {
      duration: 0.7,
      ease,
      delay: stagger(0.07),
    },
  )
}

export function fadeOut(el: Element, duration = 450): Promise<void> {
  if (reducedMotion) {
    ;(el as HTMLElement).style.opacity = '0'
    return Promise.resolve()
  }
  return new Promise((resolve) => {
    animate(
      el,
      { opacity: 0, filter: 'blur(12px)', transform: 'translateY(-18px)' },
      {
        duration: duration / 1000,
        ease,
        onComplete: () => resolve(),
      },
    )
  })
}

export function fadeIn(el: Element, duration = 700): void {
  if (reducedMotion) return
  animate(
    el,
    { opacity: 1, filter: 'blur(0px)', transform: 'translateY(0px)' },
    { duration: duration / 1000, ease },
  )
}

export function fadeVeil(veil: HTMLElement, on: boolean, duration = 220): void {
  if (reducedMotion) {
    veil.style.opacity = on ? '1' : '0'
    veil.style.pointerEvents = on ? 'auto' : 'none'
    return
  }
  animate(
    veil,
    { opacity: on ? 1 : 0 },
    {
      duration: duration / 1000,
      ease,
      onComplete: () => {
        veil.style.pointerEvents = on ? 'auto' : 'none'
      },
    },
  )
}
