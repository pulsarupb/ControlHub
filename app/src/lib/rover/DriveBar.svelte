<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import type { DriveAxis } from "$lib/rover-types"
  import { normalizedPointer } from "./drive-pointer"

  // One slider bar that owns a single drive axis: throttle (vertical) or steering (horizontal).
  let { axis }: { axis: DriveAxis } = $props()

  const vertical = $derived(axis === "throttle")
  const range = $derived(vertical ? 65 : 40)
  const value = $derived(vertical ? rover.throttle : rover.steering)

  let el: HTMLDivElement | undefined = $state()
  let pointerId: number | null = $state(null)

  function apply(event: PointerEvent): void {
    if (pointerId === null || !el || !rover.mouseJoystickActive) return
    const point = normalizedPointer(el, event)
    if (vertical) rover.setThrottle(point.y)
    else rover.setSteering(point.x)
  }

  function handleDown(event: PointerEvent): void {
    if (pointerId !== null || !el) return
    if (!rover.beginDrag([axis])) return
    pointerId = event.pointerId
    el.setPointerCapture(event.pointerId)
    apply(event)
  }

  function handleMove(event: PointerEvent): void {
    if (event.pointerId !== pointerId) return
    apply(event)
  }

  function handleUp(event: PointerEvent): void {
    if (event.pointerId !== pointerId) return
    endDrag()
  }

  function endDrag(): void {
    if (pointerId === null) return
    pointerId = null
    rover.endDrag([axis])
  }
</script>

<svelte:window onblur={endDrag} />

<div
  class="slider-area"
  class:vertical
  class:horizontal={!vertical}
  class:active={pointerId !== null}
  bind:this={el}
  role="slider"
  tabindex="0"
  aria-label={vertical ? "Forward and backward" : "Spin left and right"}
  aria-valuemin={-1}
  aria-valuemax={1}
  aria-valuenow={value}
  onpointerdown={handleDown}
  onpointermove={handleMove}
  onpointerup={handleUp}
  onpointercancel={handleUp}
  onlostpointercapture={endDrag}
>
  <div class="track"></div>
  <div
    class="puck"
    style={vertical
      ? `transform: translateY(${-value * range}px)`
      : `transform: translateX(${value * range}px)`}
  ></div>
</div>

<style>
  .slider-area {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: grid;
    place-items: center;
    border: 1px solid var(--borderStrong);
    border-radius: 8px;
    background: var(--bgDark);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.03);
    touch-action: none;
  }
  .track {
    position: absolute;
    background: var(--borderStrong);
    border-radius: 1px;
    pointer-events: none;
  }
  .vertical .track {
    width: 2px;
    height: 76%;
  }
  .horizontal .track {
    height: 2px;
    width: 76%;
  }
  .puck {
    width: 3.5rem;
    aspect-ratio: 1;
    border-radius: 50%;
    background: var(--surfaceRaised);
    border: 1px solid var(--borderStrong);
    box-shadow: 0 10px 20px rgba(0, 0, 0, 0.32);
    transition: transform 80ms ease-out;
    z-index: 1;
  }
  .slider-area.active .puck {
    transition: none;
  }
</style>
