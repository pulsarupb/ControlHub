<script lang="ts">
  import type { Template } from "$lib/types"
  import { getComponent } from "$lib/widgets/registry"

  let { t }: { t: Template } = $props()

  // Drawn from template data, so it is ready on the first frame. Text scales with the card (cqi).
  const groups = $derived(
    t.widgets.map((group) => {
      const first = group.widgets[0]
      const entry = first ? getComponent(first.widgetID, "widget") : undefined
      return {
        x: group.x,
        y: group.y,
        w: group.w,
        h: group.h,
        label: entry?.name ?? "Empty",
        extra: Math.max(group.widgets.length - 1, 0),
      }
    }),
  )
</script>

<div class="mini" aria-hidden="true">
  <div class="bar">
    {#each t.navlets as _}<i></i>{/each}
  </div>
  <div class="canvas">
    <span class="guide v" style="left:33.333%"></span>
    <span class="guide v" style="left:66.666%"></span>
    <span class="guide h"></span>
    {#each groups as g}
      <div class="group" style="--x:{g.x};--y:{g.y};--w:{g.w};--h:{g.h};">
        <span class="label">{g.label}</span>{#if g.extra > 0}<span class="extra">+{g.extra}</span>{/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .mini {
    width: 100%;
    container-type: inline-size;
    background: var(--bgDark);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    height: 1.2rem;
    padding: 0 0.45rem;
    background: var(--bgMedium);
    border-bottom: 1px solid var(--border);
  }
  .bar i {
    width: 0.9rem;
    height: 0.3rem;
    border-radius: 999px;
    background: var(--borderStrong);
  }
  /* 36 x 12 cells: three workspaces across, two rows of 6 cells each. */
  .canvas {
    position: relative;
    aspect-ratio: 36 / 12;
    width: 100%;
  }
  .guide {
    position: absolute;
    background: var(--border);
  }
  .guide.v {
    top: 0;
    bottom: 0;
    width: 1px;
  }
  .guide.h {
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
  }
  .group {
    position: absolute;
    left: calc(var(--x) / 36 * 100% + 2px);
    top: calc(var(--y) / 12 * 100% + 2px);
    width: calc(var(--w) / 36 * 100% - 4px);
    height: calc(var(--h) / 12 * 100% - 4px);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.2em;
    padding: 0 0.25em;
    overflow: hidden;
    border: 1px solid var(--borderStrong);
    border-radius: 3px;
    background: var(--bgMedium);
  }
  .label {
    min-width: 0;
    font-size: max(8px, 2.6cqi);
    line-height: 1.1;
    color: var(--textMuted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .extra {
    flex: none;
    font-size: max(8px, 2.4cqi);
    color: var(--primary);
    font-variant-numeric: tabular-nums;
  }
</style>
