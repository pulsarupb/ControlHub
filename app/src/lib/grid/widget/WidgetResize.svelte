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
    // Keep receiving moves if the finger drifts off the small handle.
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
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
      y: startY - e.clientY,
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
    width: 2.25rem;
    height: 2.25rem;
    display: grid;
    place-items: center;
    border-radius: 0.35rem;
    color: var(--textSubtle);
  }

  .resize:hover {
    color: var(--text);
  }
  /* Bigger touch target without changing the layout: the negative margin cancels the extra size. */
  @media (pointer: coarse) {
    .resize {
      box-sizing: border-box;
      width: 2.75rem;
      height: 2.75rem;
      margin: -0.25rem;
    }
  }
</style>
