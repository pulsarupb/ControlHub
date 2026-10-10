<script lang="ts">
  import { onMount } from "svelte"
  import WidgetItem from "./widget/Widget.svelte"

  import Overlay from "./Overlay.svelte"
  import PlaceOverlay from "./PlaceOverlay.svelte"
  import { setContext } from "svelte"
  import type { Manager } from "./widgets.svelte"
  import TemplatesChoose from "./TemplatesChoose.svelte"

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

  function onPointerMove(e: PointerEvent) {
    if (!manager.enableInteractions) return
    if (!section) return
    const rect = section.getBoundingClientRect()
    const mouseX = e.clientX - rect.left
    const mouseY = e.clientY - rect.top

    const x = Math.floor(mouseX / manager.WIDTH)
    const y = Math.floor(mouseY / manager.HEIGHT)

    const clampedX = Math.min(Math.max(x, 0), manager.MAX_GRID_WIDTH - 1)
    const clampedY = Math.min(Math.max(y, 0), manager.MAX_GRID_HEIGHT - 1)

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

  function onMouseLeave() {
    if (!manager.enableInteractions) return
    mouseGridPos = null
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

  let snapTimer: ReturnType<typeof setTimeout>

  onMount(() => {
    if (enableInteractions) {
      manager.loadInteractive()
    } else if (manager.widgets.length === 0 && manager.presets.data.length > 0) {
      manager.widgets = [...manager.presets.data[manager.presets.selected].widgets]
      manager.navlets = [...manager.presets.data[manager.presets.selected].navlets]
    }
    manager.updateGridSize(section, preview ? undefined : viewport)
    if (!preview) {
      // Start at the true canvas center, which is between the snap positions.
      viewport.style.scrollSnapType = "none"
      viewport.scrollTo({ left: viewport.clientWidth, top: viewport.clientHeight / 2 })
      setTimeout(() => viewport.style.removeProperty("scroll-snap-type"), 250)
    }
    if (!manager.enableInteractions || preview) return
    const handler = () => {
      const oldLeft = viewport.scrollLeft
      const oldTop = viewport.scrollTop
      manager.updateGridSize(section, viewport)
      viewport.scrollTo({ left: oldLeft, top: oldTop })
    }
    const switchHandler = (event: Event) => {
      const direction = (event as CustomEvent<string>).detail.toLowerCase()
      if (["up", "down", "left", "right"].includes(direction)) {
        navigateCanvas(direction as "up" | "down" | "left" | "right")
      }
    }
    const settleScroll = () => {
      clearTimeout(snapTimer)
      snapTimer = setTimeout(() => {
        const stepX = viewport.clientWidth
        const stepY = viewport.clientHeight
        const maxX = Math.max(0, viewport.scrollWidth - stepX)
        const maxY = Math.max(0, viewport.scrollHeight - stepY)
        const left = Math.min(Math.round(viewport.scrollLeft / stepX) * stepX, Math.round(maxX / stepX) * stepX)
        const top = Math.min(Math.round(viewport.scrollTop / stepY) * stepY, Math.round(maxY / stepY) * stepY)
        viewport.scrollTo({ left, top, behavior: "smooth" })
      }, 140)
    }
    viewport.addEventListener("scroll", settleScroll, { passive: true })
    window.addEventListener("resize", handler)
    document.addEventListener("switch-grid-nav", switchHandler)
    return () => {
      window.removeEventListener("resize", handler)
      document.removeEventListener("switch-grid-nav", switchHandler)
      viewport.removeEventListener("scroll", settleScroll)
      clearTimeout(snapTimer)
    }
  })

  let section: HTMLElement
  let viewport: HTMLElement

  function navigateCanvas(direction: "up" | "down" | "left" | "right") {
    const dx = direction === "left" ? -viewport.clientWidth : direction === "right" ? viewport.clientWidth : 0
    const dy = direction === "up" ? -viewport.clientHeight : direction === "down" ? viewport.clientHeight : 0
    viewport.scrollBy({ left: dx, top: dy, behavior: "smooth" })
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

<div
  class="wrapper"
  class:preview
  bind:this={viewport}
  data-canvas-navigation
  role="application"
  onkeydown={onCanvasKeydown}
  tabindex="0"
  aria-label="Navigable widget canvas. Use arrow keys to move between areas."
>
  <section
    class="canvas"
    bind:this={section}
    onpointermove={onPointerMove}
    onpointerleave={onMouseLeave}
    role="grid"
    tabindex="0"
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

      {#if mouseGridPos != null && !(manager.isMoving && manager.placeStart == null) && manager.tabName == ""}
        <Overlay x={mouseGridPos.x} y={mouseGridPos.y} isMouse={true} />
      {/if}
    {/if}
  </section>
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
    overflow: auto;
    flex: 1 1 auto;
    height: 0;
    min-height: 0;
    min-width: 0;
    overscroll-behavior: contain;
    -webkit-overflow-scrolling: touch;
    scroll-snap-type: both mandatory;
    scroll-padding: 0;
  }
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
      scroll-snap-align: start;
      pointer-events: none;
  }
</style>
