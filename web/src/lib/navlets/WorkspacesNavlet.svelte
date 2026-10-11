<script lang="ts">
  import { activeWorkspace, goToWorkspace, moveWorkspace } from "$lib/grid/workspace.svelte"

  let swipeStart: { pointerId: number; x: number; y: number } | null = null
  let suppressClick = false

  function removePointerListeners() {
    window.removeEventListener("pointerup", handlePointerUp)
    window.removeEventListener("pointercancel", cancelPointer)
  }

  function handlePointerDown(event: PointerEvent) {
    if (event.button !== 0) return
    removePointerListeners()
    suppressClick = false
    swipeStart = {
      pointerId: event.pointerId,
      x: event.clientX,
      y: event.clientY,
    }
    window.addEventListener("pointerup", handlePointerUp, { once: true })
    window.addEventListener("pointercancel", cancelPointer, { once: true })
  }

  function handlePointerUp(event: PointerEvent) {
    removePointerListeners()
    const start = swipeStart
    swipeStart = null
    if (!start || event.pointerId !== start.pointerId) return

    const dx = event.clientX - start.x
    const dy = event.clientY - start.y
    if (Math.max(Math.abs(dx), Math.abs(dy)) < 16) return

    suppressClick = true
    if (Math.abs(dx) >= Math.abs(dy)) {
      moveWorkspace(dx < 0 ? "right" : "left")
    } else {
      moveWorkspace(dy < 0 ? "down" : "up")
    }
  }

  function handleWorkspaceClick(column: number, row: number) {
    if (suppressClick) return
    goToWorkspace(column, row)
  }

  function handleClickCapture(event: MouseEvent) {
    if (!suppressClick) return
    event.preventDefault()
    event.stopPropagation()
    suppressClick = false
  }

  function cancelPointer() {
    removePointerListeners()
    swipeStart = null
  }
</script>

<nav
  class="workspace-nav"
  aria-label="Workspaces. Swipe to move between workspaces or tap a square to select one."
  onpointerdown={handlePointerDown}
  onclickcapture={handleClickCapture}
>
  {#each [0, 1] as row}
    <div class="workspace-row">
      {#each [0, 1, 2] as column}
        <button
          class:active={activeWorkspace.column === column && activeWorkspace.row === row}
          aria-label="Go to workspace {row * 3 + column + 1}"
          aria-current={activeWorkspace.column === column && activeWorkspace.row === row ? "page" : undefined}
          onclick={() => handleWorkspaceClick(column, row)}
        ></button>
      {/each}
    </div>
  {/each}
</nav>

<style>
  .workspace-nav {
    display: grid;
    gap: 3px;
    padding: 3px;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
  }

  .workspace-row {
    display: flex;
    gap: 3px;
  }

  button {
    width: 16px;
    height: 12px;
    padding: 0;
    border: 1px solid var(--borderStrong);
    border-radius: 3px;
    background: var(--bgLight);
  }

  button.active {
    background: var(--primary);
    border-color: var(--primary);
  }
</style>
