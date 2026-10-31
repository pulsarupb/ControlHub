<script lang="ts">
  import { onMount } from "svelte"
  import Portal from "svelte-portal"
  import { activeWorkspace, moveWorkspace, type WorkspaceDirection } from "./workspace.svelte"
  import { captureWorkspacePreviews } from "./workspace-previews"

  let { getCanvas, previewKey }: { getCanvas: () => HTMLElement | undefined; previewKey: string } = $props()
  let previews = $state<Awaited<ReturnType<typeof captureWorkspacePreviews>>>(null)
  let loading = $state(false)
  let disposed = false
  let capturedKey = ""
  let capturedAt = 0

  async function refreshPreviews(key: string) {
    const canvas = getCanvas()
    if (!canvas || loading || holding || visible || disposed) return
    if (previews && capturedKey === key && Date.now() - capturedAt < 30_000) return
    loading = true
    try {
      const result = await captureWorkspacePreviews(canvas, () => !holding && !visible && !disposed)
      if (!disposed && result) {
        previews = result
        capturedKey = key
        capturedAt = Date.now()
      }
    } catch (error) {
      console.warn("Unable to capture workspace previews:", error)
    } finally {
      if (!disposed) loading = false
    }
  }

  const directions: Record<string, WorkspaceDirection> = {
    ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right",
  }
  let visible = $state(false)
  let holding = $state(false)

  $effect(() => {
    const key = previewKey
    if (visible || holding) return
    // Never start a DOM snapshot on the keypress path. Warm the cache after
    // startup/layout changes or closing the switcher, when the browser is idle.
    let idle: number | undefined
    const timer = setTimeout(() => {
      if ("requestIdleCallback" in window) {
        idle = window.requestIdleCallback(() => { void refreshPreviews(key) })
      } else {
        void refreshPreviews(key)
      }
    }, 1200)
    return () => {
      clearTimeout(timer)
      if (idle !== undefined) window.cancelIdleCallback(idle)
    }
  })

  function routeTo(column: number, row: number) {
    const dx = column - activeWorkspace.column
    const dy = row - activeWorkspace.row
    return [
      ...Array.from({ length: Math.abs(dx) }, () => dx < 0 ? "←" : "→"),
      ...Array.from({ length: Math.abs(dy) }, () => dy < 0 ? "↑" : "↓"),
    ]
  }

  onMount(() => {
    let timer: ReturnType<typeof setTimeout> | undefined
    let suppressed = false

    function hide() {
      clearTimeout(timer)
      timer = undefined
      visible = false
    }

    function reset() {
      hide()
      holding = false
      suppressed = false
    }

    function blocked(event: KeyboardEvent) {
      return event.composedPath().some((element) => element instanceof HTMLElement && (
        element.matches("input, textarea, select") || element.isContentEditable
      )) || document.querySelector('[role="dialog"], [role="alertdialog"], dialog[open]') !== null
    }

    function keydown(event: KeyboardEvent) {
      if (event.key === "Control") {
        if (holding || event.repeat) return
        holding = true
        suppressed = blocked(event) || event.altKey || event.shiftKey || event.metaKey
        if (!suppressed) {
          timer = setTimeout(() => { visible = true }, 100)
        }
        return
      }

      if (!event.ctrlKey) return
      const direction = directions[event.key]
      if (!direction || event.altKey || event.shiftKey || event.metaKey || blocked(event)) {
        hide()
        suppressed = true
        return
      }
      if (!holding || suppressed) return
      event.preventDefault()
      event.stopImmediatePropagation()
      clearTimeout(timer)
      visible = true
      moveWorkspace(direction)
    }

    function keyup(event: KeyboardEvent) {
      if (event.key === "Control" && !event.ctrlKey) reset()
    }

    function visibilitychange() {
      if (document.hidden) reset()
    }

    // Capture before the canvas or individual widgets handle arrow keys.
    window.addEventListener("keydown", keydown, true)
    window.addEventListener("keyup", keyup, true)
    window.addEventListener("blur", reset)
    document.addEventListener("visibilitychange", visibilitychange)
    return () => {
      disposed = true
      reset()
      window.removeEventListener("keydown", keydown, true)
      window.removeEventListener("keyup", keyup, true)
      window.removeEventListener("blur", reset)
      document.removeEventListener("visibilitychange", visibilitychange)
    }
  })
</script>

{#if visible}
  <Portal>
    <div class="workspace-switcher" data-workspace-preview-ignore>
      <section aria-label="Workspace switcher">
        <h2>Workspaces</h2>
        <p>Hold <kbd>Ctrl</kbd> and use the arrow keys to move</p>
        <div class="workspace-grid">
          {#each [0, 1] as row}
            {#each [0, 1, 2] as column}
              {@const current = activeWorkspace.column === column && activeWorkspace.row === row}
              <div class="tile" class:active={current} aria-current={current ? "page" : undefined}>
                <div class="preview" style:aspect-ratio={previews?.aspectRatio ?? "16 / 9"}>
                  {#if previews}
                    <img src={previews.images[row * 3 + column]} alt="Preview of workspace {row * 3 + column + 1}" />
                  {:else}
                    <span class="preview-placeholder">Preview preparing in background</span>
                  {/if}
                </div>
                <div class="tile-caption">
                  <span class="tile-label">Workspace {row * 3 + column + 1}</span>
                  <div class="route" aria-label={current ? "Current workspace" : "Arrow keys to reach this workspace"}>
                    {#if current}
                      <span class="current">You are here</span>
                    {:else}
                      {#each routeTo(column, row) as arrow}
                        <kbd>{arrow}</kbd>
                      {/each}
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          {/each}
        </div>
        <p role="status">Workspace {activeWorkspace.row * 3 + activeWorkspace.column + 1} selected · Release Ctrl to close</p>
      </section>
    </div>
  </Portal>
{/if}

<style>
  .workspace-switcher {
    position: fixed;
    inset: 0;
    z-index: 1100;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.78);
    pointer-events: none;
  }
  section {
    width: min(1400px, 100%);
    max-height: 100%;
    overflow: auto;
    text-align: center;
    color: var(--text);
  }
  h2 { margin: 0 0 0.5rem; }
  p { color: var(--textMuted); font-size: 0.9rem; }
  .workspace-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: clamp(0.4rem, 2vw, 1rem);
    margin: 1.5rem 0;
  }
  .tile {
    display: flex;
    flex-direction: column;
    position: relative;
    overflow: hidden;
    border: 2px solid var(--borderStrong);
    border-radius: 0.75rem;
    background: var(--bgMedium);
  }
  .tile.active { border-color: var(--primary); background: var(--bgLight); }
  .preview { width: 100%; background: var(--bgDark); display: grid; place-items: center; }
  .preview img { display: block; width: 100%; height: 100%; object-fit: contain; }
  .preview-placeholder { color: var(--textMuted); font-size: 0.8rem; padding: 0.5rem; }
  .tile-caption {
    position: absolute;
    inset: auto 0 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.25rem;
    padding: 0.4rem 0.6rem;
    background: rgba(0, 0, 0, 0.82);
    color: var(--text);
  }
  .tile-label { font-size: clamp(0.7rem, 2vw, 1rem); }
  .route { display: flex; justify-content: center; align-items: center; gap: 0.25rem; min-height: 2rem; }
  kbd {
    display: inline-grid;
    place-items: center;
    min-width: 1.5em;
    padding: 0.15em 0.3em;
    border: 1px solid var(--borderStrong);
    border-bottom-width: 3px;
    border-radius: 0.3rem;
    background: var(--bgDark);
    color: var(--text);
    font-family: inherit;
  }
  .route kbd { font-size: clamp(0.9rem, 2.5vw, 1.4rem); }
  .current { color: var(--primary); font-size: 0.85rem; }
</style>
