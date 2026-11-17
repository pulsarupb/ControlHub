<script lang="ts">
  import IconTrashRegular from "phosphor-icons-svelte/IconTrashRegular.svelte"
  import IconXRegular from "phosphor-icons-svelte/IconXRegular.svelte"
  import { getContext } from "svelte"
  import { confirmDelete } from "$lib/ui/confirm.svelte"
  import type { ExtendedWidgetGroup, Manager } from "../widgets.svelte"
  const manager = getContext("manager") as Manager

  let { widget = $bindable() }: { widget: ExtendedWidgetGroup } = $props()

  let hasSelected = $derived(widget.selected >= 0 && widget.widgets.length > 0)

  // Removing the selected widget empties its tab; the tab itself is removed only when it is not the last one.
  function removeSelected() {
    if (!hasSelected) return
    if (widget.widgets.length === 1) {
      widget.widgets[0] = { isMoving: false, widgetID: "" }
      return
    }
    widget.widgets.splice(widget.selected, 1)
    if (widget.selected >= widget.widgets.length) widget.selected = widget.widgets.length - 1
  }
</script>

<div class="options">
  <button
    class="icon-btn"
    aria-label="Remove this widget"
    title="Remove this widget"
    disabled={!hasSelected}
    onclick={() => confirmDelete("Remove widget?", "This removes the widget from its group.", removeSelected)}
  >
    <IconXRegular />
  </button>
  <button
    class="icon-btn danger"
    aria-label="Delete group"
    title="Delete group"
    onclick={() => confirmDelete("Delete group?", `This deletes the group and its ${widget.widgets.length} widget${widget.widgets.length === 1 ? "" : "s"}.`, () => manager.removeWidget(widget.id))}
  >
    <IconTrashRegular />
  </button>
</div>

<style>
  .options { display: flex; align-items: center; gap: 2px; }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 1.6rem;
    height: 1.6rem;
    padding: 0;
    border: 0;
    border-radius: 0.35rem;
    background: transparent;
    color: var(--textMuted);
    cursor: pointer;
  }
  .icon-btn:hover:not(:disabled),
  .icon-btn:focus-visible { background: var(--bgLight); color: var(--text); }
  .icon-btn.danger:hover:not(:disabled) { color: var(--danger); }
</style>
