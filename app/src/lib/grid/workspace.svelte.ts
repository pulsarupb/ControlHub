export type WorkspaceDirection = "up" | "down" | "left" | "right"

export const activeWorkspace = $state({ column: 1, row: 0 })

const DURATION = 300 // ms

let viewport: HTMLElement | null = null
let frame: number | null = null

function cancelAnimation() {
  if (frame !== null) {
    cancelAnimationFrame(frame)
    frame = null
  }
}

function easeOutCubic(t: number) {
  return 1 - Math.pow(1 - t, 3)
}

export function bindWorkspaceViewport(element: HTMLElement | null) {
  viewport = element
  if (viewport) goToWorkspace(activeWorkspace.column, activeWorkspace.row, false)
  else cancelAnimation()
}

export function goToWorkspace(column: number, row: number, smooth = true) {
  activeWorkspace.column = Math.max(0, Math.min(2, column))
  activeWorkspace.row = Math.max(0, Math.min(1, row))
  if (!viewport) return

  const left = activeWorkspace.column * viewport.clientWidth
  const top = activeWorkspace.row * viewport.clientHeight

  cancelAnimation()

  if (!smooth) {
    viewport.scrollTo({ left, top, behavior: "instant" })
    return
  }

  // Animate scroll ourselves: Chromium disables native smooth scrolling when
  // the OS animation setting is off, which made switching instant on desktop.
  const fromLeft = viewport.scrollLeft
  const fromTop = viewport.scrollTop
  if (Math.abs(left - fromLeft) < 1 && Math.abs(top - fromTop) < 1) return

  const started = performance.now()
  const step = (now: number) => {
    const progress = Math.min(1, (now - started) / DURATION)
    const eased = easeOutCubic(progress)
    viewport!.scrollLeft = fromLeft + (left - fromLeft) * eased
    viewport!.scrollTop = fromTop + (top - fromTop) * eased
    frame = progress < 1 ? requestAnimationFrame(step) : null
  }
  frame = requestAnimationFrame(step)
}

export function moveWorkspace(direction: WorkspaceDirection) {
  const column = activeWorkspace.column + (direction === "left" ? -1 : direction === "right" ? 1 : 0)
  const row = activeWorkspace.row + (direction === "up" ? -1 : direction === "down" ? 1 : 0)
  goToWorkspace(column, row)
}
