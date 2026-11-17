<script lang="ts">
  import type { Template } from "$lib/types"
  import { setContext } from "svelte"
  import { getComponent } from "$lib/widgets/registry"
  import { activeWorkspace } from "./workspace.svelte"
  import WidgetContent from "./WidgetContent.svelte"

  // A template drawn like the full screen, zoomed out: a 16:9 frame with the top bar and the
  // whole 36 x 12 canvas (six workspaces) inside. The workspace you are on is outlined.
  let { t }: { t: Template } = $props()

  // Widgets that check this skip their 3D scenes, polling and render loops: a preview is a picture, not a live view.
  setContext("widget-preview", true)

  // Design frame at 16:9. The canvas fills everything under the top bar edge to edge.
  // Cells are stretched to the frame: wider than tall, but widget content keeps its shape.
  const FRAME_W = 1600
  const FRAME_H = 900
  const BAR_H = 56
  const CANVAS_W = FRAME_W
  const CANVAS_H = FRAME_H - BAR_H
  const CELL_X = CANVAS_W / 36
  const CELL_Y = CANVAS_H / 12

  let host: HTMLDivElement
  let scale = $state(0.4)

  $effect(() => {
    const observer = new ResizeObserver(() => {
      scale = host.clientWidth / FRAME_W
    })
    observer.observe(host)
    return () => observer.disconnect()
  })

  const groups = $derived(
    t.widgets.map((group) => ({
      x: group.x,
      y: group.y,
      w: group.w,
      h: group.h,
      tabs: group.widgets.map((w) => ({ id: w.widgetID, name: getComponent(w.widgetID, "widget")?.name ?? "Empty" })),
    })),
  )

  const workspaces = [0, 1, 2, 3, 4, 5]
</script>

<div class="live" bind:this={host} aria-hidden="true" style:height="{scale * FRAME_H}px">
  <div class="stage" style:width="{FRAME_W}px" style:height="{FRAME_H}px" style:transform="scale({scale})">
    <div class="bar">
      <span class="logo"></span>
      <span class="pill">offline / drive stopped</span>
      <span class="icons"><i></i><i></i><i></i><i></i></span>
    </div>

    <div class="canvas" style:left="0" style:top="{BAR_H}px" style:width="{CANVAS_W}px" style:height="{CANVAS_H}px">
      {#each workspaces as n}
        {@const column = n % 3}
        {@const row = Math.floor(n / 3)}
        {@const current = activeWorkspace.column === column && activeWorkspace.row === row}
        <div
          class="workspace"
          class:current
          style:left="{column * 12 * CELL_X}px"
          style:top="{row * 6 * CELL_Y}px"
          style:width="{12 * CELL_X}px"
          style:height="{6 * CELL_Y}px"
        ></div>
      {/each}

      {#each groups as g}
        <div class="group" style:left="{g.x * CELL_X}px" style:top="{g.y * CELL_Y}px" style:width="{g.w * CELL_X}px" style:height="{g.h * CELL_Y}px">
          <div class="tabs">
            {#each g.tabs as tab, i}
              <span class="tab" class:on={i === 0}>{tab.name}</span>
            {/each}
          </div>
          <div class="body">
            {#if g.tabs[0]}
              <WidgetContent widgetID={g.tabs[0].id} />
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .live {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 0.35rem;
    background: var(--bgDark);
    pointer-events: none;
  }
  .stage {
    position: absolute;
    top: 0;
    left: 0;
    transform-origin: top left;
    background: var(--bgDark);
  }
  .bar {
    position: absolute;
    inset: 0 0 auto 0;
    height: 56px;
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 0 28px;
    border-bottom: 1px solid var(--border);
    background: var(--bgMedium);
  }
  .logo {
    width: 40px;
    height: 36px;
    border: 3px solid var(--textMuted);
    border-radius: 6px;
  }
  .pill {
    padding: 6px 16px;
    border: 1px solid var(--primary);
    border-radius: 6px;
    color: var(--primary);
    font-size: 20px;
  }
  .icons {
    margin-left: auto;
    display: flex;
    gap: 14px;
  }
  .icons i {
    width: 26px;
    height: 26px;
    border: 2px solid var(--textSubtle);
    border-radius: 6px;
  }
  .canvas {
    position: absolute;
  }
  .workspace {
    position: absolute;
    box-sizing: border-box;
    border: 1px dashed var(--border);
  }
  .workspace.current {
    border: 4px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    z-index: 1;
  }
  .group {
    position: absolute;
    box-sizing: border-box;
    padding: 6px;
  }
  .group::before {
    content: "";
    position: absolute;
    inset: 6px;
    border: 1px solid var(--borderStrong);
    border-radius: 8px;
    background: var(--bgMedium);
    pointer-events: none;
  }
  .tabs {
    position: relative;
    display: flex;
    gap: 4px;
    height: 28px;
    align-items: center;
    padding: 0 10px;
    overflow: hidden;
    white-space: nowrap;
  }
  .tab {
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--bgLight);
    color: var(--textMuted);
    font-size: 13px;
  }
  .tab.on {
    background: var(--surfaceRaised);
    color: var(--text);
    outline: 1px solid var(--borderStrong);
  }
  .body {
    position: absolute;
    left: 12px;
    right: 12px;
    top: 40px;
    bottom: 12px;
    overflow: hidden;
    border-radius: 6px;
  }
</style>
