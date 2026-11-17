<script lang="ts">
  import IconArrowsOutCardinalRegular from "phosphor-icons-svelte/IconArrowsOutCardinalRegular.svelte"
  import { getContext } from "svelte"
  import { type ExtendedWidgetGroup } from "../widgets.svelte"
  import type { Manager } from "../widgets.svelte"
  const manager = getContext("manager") as Manager
  let {
    widget = $bindable(),
    isPossible = $bindable(),
  }: { widget: ExtendedWidgetGroup; isPossible: boolean } = $props()

  let startX = $state(0)
  let startY = $state(0)

  let pointerId: number | null = null
  // Pointer moves can arrive faster than frames: recompute the layout at most once per frame.
  let frame: number | null = null

  function scheduleMove() {
    if (frame !== null) return
    frame = requestAnimationFrame(() => {
      frame = null
      manager.updateMove(widget.id)
    })
  }

  function startDrag(e: PointerEvent) {
    if (!manager.enableInteractions) return
    if (isPossible) return

    widget.isMoving = true
    pointerId = e.pointerId
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
    widget.move = {
      x: 0,
      y: 0,
    }
    e.preventDefault()
    startX = e.clientX
    startY = e.clientY
    manager.updateMove(widget.id)
    window.addEventListener("pointermove", onDrag)
    window.addEventListener("pointerup", stopDrag)
    window.addEventListener("pointercancel", stopDrag)
  }

  function onDrag(e: PointerEvent) {
    if (!manager.enableInteractions || e.pointerId !== pointerId) return
    widget.move = {
      x: e.clientX - startX,
      y: e.clientY - startY,
    }
    scheduleMove()
  }

  function stopDrag(e: PointerEvent) {
    if (!manager.enableInteractions || e.pointerId !== pointerId) return
    if (frame !== null) { cancelAnimationFrame(frame); frame = null }
    widget.isMoving = false
    window.removeEventListener("pointermove", onDrag)
    window.removeEventListener("pointerup", stopDrag)
    window.removeEventListener("pointercancel", stopDrag)
    pointerId = null
    manager.finishMoveWidget(widget.id)
    widget.move = {
      x: 0,
      y: 0,
    }
  }
</script>

<button data-workspace-preview-ignore class="icon" onpointerdown={startDrag}><IconArrowsOutCardinalRegular /></button>

<style>
  .icon {
    all: unset;
    cursor: pointer;
    touch-action: none;
    width: fit-content;
    display: flex;
  }
  /* Larger touch target; padding is cancelled by the negative margin so the top bar layout does not change. */
  @media (pointer: coarse) {
    .icon {
      padding: 0.5rem;
      margin: -0.5rem;
    }
  }
</style>
