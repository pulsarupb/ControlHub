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
        <header>
          <h2>Workspaces</h2>
          <p>Hold <kbd>Ctrl</kbd> + arrows</p>
        </header>
        <div class="workspace-grid">
          {#each [0, 1] as row}
            {#each [0, 1, 2] as column}
              {@const current = activeWorkspace.column === column && activeWorkspace.row === row}
              <div class="tile" class:active={current} aria-current={current ? "page" : undefined}>
                <div class="preview" style:aspect-ratio={previews?.aspectRatio ?? "16 / 9"}>
                  {#if previews}
                    <img src={previews.images[row * 3 + column]} alt="Preview of workspace {row * 3 + column + 1}" />
                  {:else}
                    <span class="preview-placeholder" aria-hidden="true"></span>
                  {/if}
                </div>
                <div class="tile-caption">
                  <span class="tile-label">{row * 3 + column + 1}</span>
                  <div class="route" aria-label={current ? "Current workspace" : "Arrow keys to reach this workspace"}>
                    {#if current}
                      <span class="current">Here</span>
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
        <p role="status">Workspace {activeWorkspace.row * 3 + activeWorkspace.column + 1} · release Ctrl to close</p>
      </section>
    </div>
  </Portal>
{/if}

<style>
  /* Compact floating panel anchored to the bottom edge. Nothing covers the page behind it. */
  .workspace-switcher {
    position: fixed;
    left: 50%;
    bottom: 1rem;
    z-index: 1100;
    transform: translateX(-50%);
    width: min(380px, calc(100vw - 2rem));
    pointer-events: none;
  }
  section {
    display: grid;
    gap: 0.5rem;
    padding: 0.6rem;
    border: 1px solid var(--borderStrong);
    border-radius: 0.6rem;
    background: var(--surfaceRaised);
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.45);
    color: var(--text);
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
  }
  h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
  }
  p {
    margin: 0;
    color: var(--textMuted);
    font-size: 0.75rem;
  }
  header p {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .workspace-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.35rem;
  }
  .tile {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 0.4rem;
    background: var(--bgDark);
  }
  .tile.active {
    border-color: var(--primary);
  }
  .preview {
    display: grid;
    place-items: center;
    width: 100%;
    background: var(--bgMedium);
  }
  .preview img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  /* Neutral slot while the snapshot is captured: no text, so an empty tile does not read as an error. */
  .preview-placeholder {
    display: block;
    width: 100%;
    height: 100%;
    min-height: 3rem;
    background: var(--bgLight);
    opacity: 0.35;
  }
  .tile-caption {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.25rem;
    padding: 0.25rem 0.35rem;
    border-top: 1px solid var(--border);
    font-size: 0.7rem;
  }
  .tile-label {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .tile.active .tile-label {
    color: var(--primary);
  }
  .route {
    display: flex;
    align-items: center;
    gap: 0.15rem;
    min-height: 1rem;
  }
  kbd {
    display: inline-grid;
    place-items: center;
    min-width: 1.2em;
    padding: 0 0.2em;
    border: 1px solid var(--borderStrong);
    border-radius: 0.25rem;
    background: var(--bgLight);
    color: var(--text);
    font-family: inherit;
    font-size: 0.65rem;
  }
  .current {
    color: var(--primary);
    font-weight: 600;
  }
</style>
