<script lang="ts">
  import { onMount } from "svelte"
  import WidgetItem from "./widget/Widget.svelte"

  import Overlay from "./Overlay.svelte"
  import WorkspaceSwitcher from "./WorkspaceSwitcher.svelte"
  import PlaceOverlay from "./PlaceOverlay.svelte"
  import { setContext } from "svelte"
  import type { Manager } from "./widgets.svelte"
import TemplatesChoose from "./TemplatesChoose.svelte"
  import { bindWorkspaceViewport, moveWorkspace, swipeToWorkspace } from "./workspace.svelte"

  let {
    manager = $bindable(),
    enableInteractions = false,
    preview = false,
  }: {
    manager: Manager
    enableInteractions?: boolean
    preview?: boolean
  } = $props()

  setContext("manager", manager)

  let mouseGridPos = $state<{
    x: number
    y: number
    isInsideWidget: boolean
  } | null>(null)

  // Background swipe on touch: starts only when the finger goes down on empty canvas.
  let swipeStart: { id: number; x: number; y: number } | null = null

  function cellAt(clientX: number, clientY: number) {
    const rect = section.getBoundingClientRect()
    const x = Math.floor((clientX - rect.left) / manager.WIDTH)
    const y = Math.floor((clientY - rect.top) / manager.HEIGHT)
    return {
      x: Math.min(Math.max(x, 0), manager.MAX_GRID_WIDTH - 1),
      y: Math.min(Math.max(y, 0), manager.MAX_GRID_HEIGHT - 1),
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (!manager.enableInteractions) return
    if (!section) return
    const { x: clampedX, y: clampedY } = cellAt(e.clientX, e.clientY)

    // Canvas-level so the rectangle keeps growing while a touch drag is captured by the "+" button.
    if (manager.placeStart != null) manager.updatePlace(clampedX, clampedY)

    if (manager.getWidget(clampedX, clampedY, manager.widgets) != undefined) {
      if (manager.placeStart != null) {
        mouseGridPos = { x: clampedX, y: clampedY, isInsideWidget: true }
      } else {
        mouseGridPos = null
      }
      return
    }

    mouseGridPos = { x: clampedX, y: clampedY, isInsideWidget: false }
  }

  // Touch pointers fire pointerleave when the finger lifts, which would hide the "+" before it can be tapped.
  function onMouseLeave(e: PointerEvent) {
    if (!manager.enableInteractions) return
    if (e.pointerType !== "mouse") return
    mouseGridPos = null
  }

  function onCanvasPointerDown(e: PointerEvent) {
    if (!manager.enableInteractions || e.pointerType === "mouse" || !e.isPrimary) return
    // Tap an empty cell to show the "+" under the finger.
    if (section) {
      const { x, y } = cellAt(e.clientX, e.clientY)
      const onWidget = manager.getWidget(x, y, manager.widgets) != undefined
      if (onWidget) {
        if (manager.placeStart == null) mouseGridPos = null
      } else if (manager.placeStart == null) {
        mouseGridPos = { x, y, isInsideWidget: false }
      }
    }
    swipeStart = e.target === section ? { id: e.pointerId, x: e.clientX, y: e.clientY } : null
  }

  function onCanvasPointerUp(e: PointerEvent) {
    if (!swipeStart || e.pointerId !== swipeStart.id) return
    const start = swipeStart
    swipeStart = null
    if (swipeToWorkspace(e.clientX - start.x, e.clientY - start.y)) mouseGridPos = null
  }

  let gridCells = $derived.by(() => {
    let list = []
    for (let y = 0; y < manager.MAX_GRID_HEIGHT; y++) {
      for (let x = 0; x < manager.MAX_GRID_WIDTH; x++) {
        list.push({ x, y })
      }
    }
    return list
  })

  onMount(() => {
    if (enableInteractions) {
      manager.loadInteractive()
    } else if (manager.widgets.length === 0 && manager.presets.data.length > 0) {
      manager.widgets = [...manager.presets.data[manager.presets.selected].widgets]
      manager.navlets = [...manager.presets.data[manager.presets.selected].navlets]
    }
    manager.updateGridSize(section, preview ? undefined : viewport)
    if (!preview) bindWorkspaceViewport(viewport)
    if (!manager.enableInteractions || preview) return
    const handler = () => {
      manager.updateGridSize(section, viewport)
      bindWorkspaceViewport(viewport)
    }
    window.addEventListener("resize", handler)
    return () => {
      window.removeEventListener("resize", handler)
      bindWorkspaceViewport(null)
    }
  })

  let section: HTMLElement
  let viewport: HTMLElement

  function navigateCanvas(direction: "up" | "down" | "left" | "right") {
    moveWorkspace(direction)
  }

  function onCanvasKeydown(event: KeyboardEvent) {
    const directions: Record<string, "up" | "down" | "left" | "right"> = {
      ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right",
    }
    const direction = directions[event.key]
    if (!direction || !manager.enableInteractions) return
    event.preventDefault()
    navigateCanvas(direction)
  }
</script>

{#if enableInteractions && !preview}
  <WorkspaceSwitcher
    getCanvas={() => section}
    previewKey={JSON.stringify([manager.WIDTH, manager.HEIGHT, manager.widgets.map((widget) => [
      widget.id, widget.x, widget.y, widget.w, widget.h, widget.selected,
      widget.widgets.map((panel) => panel.widgetID),
    ])])}
  />
{/if}

<div
  class="wrapper"
  class:preview
  bind:this={viewport}
  data-canvas-navigation
>
  <div
    class="canvas"
    bind:this={section}
    onpointerdown={onCanvasPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onCanvasPointerUp}
    onpointercancel={() => (swipeStart = null)}
    onpointerleave={onMouseLeave}
    onkeydown={onCanvasKeydown}
    role="grid"
    tabindex="0"
    aria-label="Widget canvas. Use arrow keys to change workspaces."
    style="--width:{manager.WIDTH}px;--wCount:{manager.MAX_GRID_WIDTH};--height:{manager.HEIGHT}px;--hCount:{manager.MAX_GRID_HEIGHT};width:{preview ? '100%' : `${manager.WIDTH * manager.MAX_GRID_WIDTH}px`};height:{preview ? '100%' : `${manager.HEIGHT * manager.MAX_GRID_HEIGHT}px`};"
  >
    {#each [0, 1, 2] as column}
      {#each [0, 1] as row}
      <div
        class="snap-page"
        aria-hidden="true"
        style="--x:{column * 12};--y:{row * 6};"
      ></div>
      {/each}
    {/each}
    {#if manager.isEmpty}
      <div
        class="item"
        style="--w:{manager.MAX_GRID_WIDTH};--h:{manager.MAX_GRID_HEIGHT};"
      >
        <TemplatesChoose
          set={(t) => {
            manager.replaceCurrentWith(t)
          }}
        />
      </div>
    {:else}
      {#if manager.isMoving}
        {#each gridCells as { x, y } (x + "-" + y)}
          <Overlay {x} {y} />
        {/each}
        {#each manager.possibleWidgets as widget, index}
          <WidgetItem
            isPossible={true}
            bind:widget={manager.possibleWidgets[index]}
          />
        {/each}
      {/if}
      <PlaceOverlay />

      {#each manager.widgets as widget, index}
        {#if !manager.isMoving || widget.isMoving}
          <WidgetItem isPossible={false} bind:widget={manager.widgets[index]} />
        {/if}
      {/each}

      {#if mouseGridPos != null && (manager.placeStart != null || manager.getWidget(mouseGridPos.x, mouseGridPos.y, manager.widgets) == undefined) && !(manager.isMoving && manager.placeStart == null) && manager.tabName == ""}
        <Overlay x={mouseGridPos.x} y={mouseGridPos.y} isMouse={true} />
      {/if}
    {/if}
  </div>
</div>

<style>
  .item {
    position: absolute;
    top: 0;
    left: 0;
    height: calc(var(--h) * var(--height));
    width: calc(var(--w) * var(--width));
    display: grid;
    place-items: center;
    flex-direction: column;
    padding: var(--spacing);
  }
  .wrapper {
    position: relative;
    overflow: hidden;
    flex: 1 1 auto;
    height: 0;
    min-height: 0;
    min-width: 0;
    overscroll-behavior: contain;
    scrollbar-width: none;
    -ms-overflow-style: none;
    /* The viewport never scrolls natively; disabling browser panning lets background swipes reach the pointer handlers. */
    touch-action: none;
  }
  .wrapper::-webkit-scrollbar { display: none; }
  .wrapper.preview {
    overflow: hidden;
    width: 100%;
    height: 100%;
  }
  .canvas {
    --spacing: 0.25rem;
    position: relative;

    overflow: visible;
    margin: 0;
    min-width: 0;
    flex: none;
  }
  .wrapper.preview .canvas {
    margin: 0;
    min-width: 0;
    width: 100%;
    height: 100%;
    width: 100%;
  }

  .snap-page {
      position: absolute;
      top: calc(var(--y) * var(--height));
      left: calc(var(--x) * var(--width));
      width: calc(12 * var(--width));
      height: calc(6 * var(--height));
      scroll-snap-align: none;
      pointer-events: none;
  }

</style>
