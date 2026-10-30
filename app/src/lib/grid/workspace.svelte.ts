export type WorkspaceDirection = "up" | "down" | "left" | "right"

export const activeWorkspace = $state({ column: 1, row: 0 })

let viewport: HTMLElement | null = null

export function bindWorkspaceViewport(element: HTMLElement | null) {
  viewport = element
  if (viewport) goToWorkspace(activeWorkspace.column, activeWorkspace.row, false)
}

export function goToWorkspace(column: number, row: number, smooth = true) {
  activeWorkspace.column = Math.max(0, Math.min(2, column))
  activeWorkspace.row = Math.max(0, Math.min(1, row))
  viewport?.scrollTo({
    left: activeWorkspace.column * viewport.clientWidth,
    top: activeWorkspace.row * viewport.clientHeight,
    behavior: smooth ? "smooth" : "instant",
  })
}

export function moveWorkspace(direction: WorkspaceDirection) {
  const column = activeWorkspace.column + (direction === "left" ? -1 : direction === "right" ? 1 : 0)
  const row = activeWorkspace.row + (direction === "up" ? -1 : direction === "down" ? 1 : 0)
  goToWorkspace(column, row)
}
