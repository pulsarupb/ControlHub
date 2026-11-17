<script lang="ts">
  import WidgetTopBar from "./WidgetTopBar.svelte"
  import WidgetChoose from "./WidgetChoose.svelte"
  import WidgetResize from "./WidgetResize.svelte"
  import WidgetOptions from "./WidgetOptions.svelte"
  import { getContext } from "svelte"
  import type { ExtendedWidgetGroup, Manager } from "../widgets.svelte"
  import WidgetContent from "../WidgetContent.svelte"
  const manager = getContext("manager") as Manager

  let {
    widget = $bindable(),
    isPossible,
  }: { widget: ExtendedWidgetGroup; isPossible: boolean } = $props()
  let isSmall = $derived(widget.w == 1 && widget.h == 1)
  // Half a gap per side; a side on a workspace or canvas edge (every 12 columns, every 6 rows) takes the other half.
  const pad = (edge: boolean) => (edge ? "var(--gap)" : "calc(var(--gap) / 2)")
  let padStyle = $derived(
    `--padL:${pad(widget.x % 12 === 0)};--padR:${pad((widget.x + widget.w) % 12 === 0)};` +
    `--padT:${pad(widget.y % 6 === 0)};--padB:${pad((widget.y + widget.h) % 6 === 0)};`,
  )
</script>

<div
  class="item"
  class:transparent={widget.isMoving && !isPossible}
  style="{padStyle}--x:{widget.x};--y:{widget.y};--w:{widget.w};--h:{widget.h};--xOffset:{widget
    .offset.x}px;--yOffset:{widget.offset.y}px;--xMove:{widget.move
    .x}px;--yMove:{widget.move.y}px;"
>
  <div
    class="content"
    class:invalid={!manager.isValid &&
      !isPossible &&
      manager.isMoving &&
      manager.placeStart == null}
  >
    <WidgetTopBar bind:widget bind:isPossible />
    <section>
      {#if widget.widgets.length > 0}
        {#each widget.widgets, index}
          <div
            class="wrapper"
            style={index == widget.selected
              ? "display:block;"
              : "display:none;"}
          >
            {#if widget.widgets[index].widgetID != "" && manager.exists(widget.widgets[index].widgetID)}
              <WidgetContent
                widgetID={widget.widgets[index].widgetID}
              />
            {:else}
              <WidgetChoose
                set={(wID) => {
                  widget.widgets[index].widgetID = wID
                }}
              />
            {/if}
          </div>
        {/each}
      {:else}
        <WidgetChoose
          set={(wID) => {
            widget.widgets.push({
              isMoving: false,
              widgetID: wID,
            })
          }}
        />
      {/if}
    </section>
  </div>
  <!-- Narrow widgets: the top bar has no room for its controls, so resize and the menu stack in the top-right corner, outside the clipped .content. Never at the bottom: bottom-edge drags close the app on some devices. -->
  <div class="corner-grip" data-workspace-preview-ignore>
    <WidgetResize bind:widget {isPossible} />
    <WidgetOptions bind:widget />
  </div>
</div>

<style>
  .wrapper {
    height: 100%;
  }
  section {
    padding: var(--inset);
    overflow: auto;
    flex-grow: 1;
  }
  .item {
    position: absolute;
    top: calc(var(--y) * var(--height) + var(--yMove) - var(--yOffset));
    left: calc(var(--x) * var(--width) + var(--xMove));
    height: calc(var(--h) * var(--height) + var(--yOffset));
    width: calc(var(--w) * var(--width) + var(--xOffset));
    display: flex;
    flex-direction: column;
    padding: var(--padT, calc(var(--gap) / 2)) var(--padR, calc(var(--gap) / 2)) var(--padB, calc(var(--gap) / 2)) var(--padL, calc(var(--gap) / 2));
    container-type: inline-size;
  }
  .corner-grip {
    display: none;
    position: absolute;
    /* Inside the widget, not straddling its edge: the viewport clips anything above the top row. */
    top: var(--spacing);
    right: 0;
    z-index: 2;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 2px;
    background: var(--bgMedium);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
  }
  /* Same threshold as WidgetTopBar: below it, the top-bar controls are hidden and this stack shows. */
  @container (max-width: 180px) {
    .corner-grip {
      display: flex;
    }
    /* Keep the header's right-hand controls clear of the corner stack. */
    .content :global(.widget-header) {
      padding-right: 1.9rem;
    }
  }
  .content {
    position: relative;
    flex-grow: 1;
    background-color: var(--bgMedium);
    border: 1px solid var(--border);
    overflow: hidden;

    display: flex;
    flex-direction: column;

    border-radius: 0.55rem;
  }
  .content.invalid {
    background-color: color-mix(in srgb, var(--danger) 20%, transparent);
    border-color: var(--danger);
  }
  .transparent {
    opacity: 0.5;
  }
</style>
