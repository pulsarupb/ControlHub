<script lang="ts">
  import IconArrowsOutSimpleRegular from "phosphor-icons-svelte/IconArrowsOutSimpleRegular.svelte"
  import { getContext } from "svelte"
  import type { ExtendedWidgetGroup, Manager } from "../widgets.svelte"
  const manager = getContext("manager") as Manager
  let {
    widget = $bindable(),
    isPossible = $bindable(),
  }: { widget: ExtendedWidgetGroup; isPossible: boolean } = $props()

  let startX = $state(0)
  let startY = $state(0)

  function startResize(e: MouseEvent) {
    if (!manager.enableInteractions) return
    if (isPossible) return
    widget.isMoving = true
    widget.offset = {
      x: 0,
      y: 0,
    }
    startX = e.clientX
    startY = e.clientY
    manager.updateResize(widget.id)
    window.addEventListener("mousemove", onResize)
    window.addEventListener("mouseup", stopResize)
  }

  function onResize(e: MouseEvent) {
    if (!manager.enableInteractions) return
    e.preventDefault()
    widget.offset = {
      x: e.clientX - startX,
      y: e.clientY - startY,
    }

    manager.updateResize(widget.id)
  }

  function stopResize() {
    if (!manager.enableInteractions) return
    widget.isMoving = false
    window.removeEventListener("mousemove", onResize)
    window.removeEventListener("mouseup", stopResize)
    manager.finishResizeWidget(widget.id)
    widget.offset = {
      x: 0,
      y: 0,
    }
  }
</script>

<button class="icon resize" onmousedown={startResize}>
  <IconArrowsOutSimpleRegular />
</button>

<style>
  .icon {
    all: unset;
    cursor: pointer;
    width: fit-content;
    display: flex;
  }
  .resize {
    position: absolute;
    right: 0.35rem;
    bottom: 0.35rem;
    color: var(--textSubtle);
  }
</style>
