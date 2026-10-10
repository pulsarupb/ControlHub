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

  let pointerId: number | null = null

  function startResize(e: PointerEvent) {
    if (!manager.enableInteractions) return
    if (isPossible) return
    widget.isMoving = true
    pointerId = e.pointerId
    widget.offset = {
      x: 0,
      y: 0,
    }
    startX = e.clientX
    startY = e.clientY
    manager.updateResize(widget.id)
    window.addEventListener("pointermove", onResize)
    window.addEventListener("pointerup", stopResize)
    window.addEventListener("pointercancel", stopResize)
  }

  function onResize(e: PointerEvent) {
    if (!manager.enableInteractions || e.pointerId !== pointerId) return
    e.preventDefault()
    widget.offset = {
      x: e.clientX - startX,
      y: e.clientY - startY,
    }

    manager.updateResize(widget.id)
  }

  function stopResize(e: PointerEvent) {
    if (!manager.enableInteractions || e.pointerId !== pointerId) return
    widget.isMoving = false
    window.removeEventListener("pointermove", onResize)
    window.removeEventListener("pointerup", stopResize)
    window.removeEventListener("pointercancel", stopResize)
    pointerId = null
    manager.finishResizeWidget(widget.id)
    widget.offset = {
      x: 0,
      y: 0,
    }
  }
</script>

<button class="icon resize" onpointerdown={startResize}>
  <IconArrowsOutSimpleRegular />
</button>

<style>
  .icon {
    all: unset;
    cursor: pointer;
    touch-action: none;
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
