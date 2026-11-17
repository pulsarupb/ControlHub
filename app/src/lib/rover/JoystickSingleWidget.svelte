<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import { normalizedPointer } from "./drive-pointer"

  // One 2D pad: up/down is throttle, left/right is steering. Holding a diagonal drives and turns at once.
  let padEl: HTMLDivElement | undefined = $state()
  let pointerId: number | null = $state(null)

  function apply(event: PointerEvent): void {
    if (pointerId === null || !padEl || !rover.mouseJoystickActive) return
    const point = normalizedPointer(padEl, event)
    // Clamp to the unit circle so the corners do not exceed full throttle/steer combined.
    const magnitude = Math.hypot(point.x, point.y)
    const scale = magnitude > 1 ? 1 / magnitude : 1
    rover.setSteering(point.x * scale);
    rover.setThrottle(point.y * scale);
  }

  function handleDown(event: PointerEvent): void {
    if (pointerId !== null || !padEl) return
    if (!rover.beginDrag(["throttle", "steering"])) return
    pointerId = event.pointerId
    padEl.setPointerCapture(event.pointerId)
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
    rover.endDrag(["throttle", "steering"])
  }

  const knobLeft = $derived(50 + rover.steering * 50)
  const knobTop = $derived(50 - rover.throttle * 50)
</script>

<svelte:window onblur={endDrag} />

<div class="joystick-single">
  <strong class="readout">T {rover.throttle.toFixed(2)} / S {rover.steering.toFixed(2)}</strong>

  <div class="pad-frame">
    <div
      class="pad"
      class:active={pointerId !== null}
      bind:this={padEl}
      role="slider"
      tabindex="0"
      aria-label="Drive joystick: up and down throttle, left and right steering"
      aria-valuemin={-1}
      aria-valuemax={1}
      aria-valuenow={rover.throttle}
      aria-valuetext={`Throttle ${rover.throttle.toFixed(2)}, steering ${rover.steering.toFixed(2)}`}
      onpointerdown={handleDown}
      onpointermove={handleMove}
      onpointerup={handleUp}
      onpointercancel={handleUp}
      onlostpointercapture={endDrag}
    >
      <div class="cross cross-v"></div>
      <div class="cross cross-h"></div>
      <div class="knob" style={`left: ${knobLeft}%; top: ${knobTop}%`}></div>
    </div>
  </div>

  <div class="labels">
    <span>Throttle: up / down</span>
    <span>Steer: left / right</span>
  </div>
</div>

<style>
  .joystick-single {
    min-height: 100%;
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 0.5rem;
    overflow-x: hidden;
    overflow-y: auto;
  }
  .readout {
    color: var(--accent);
    font-size: 0.82rem;
    text-align: right;
    text-transform: uppercase;
  }
  .pad-frame {
    container-type: size;
    display: grid;
    place-items: center;
    min-height: 0;
    min-width: 0;
  }
  .pad {
    position: relative;
    width: min(100cqw, 100cqh);
    height: min(100cqw, 100cqh);
    border: 1px solid var(--borderStrong);
    border-radius: 50%;
    background: var(--bgDark);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.03);
    touch-action: none;
  }
  .cross {
    position: absolute;
    background: var(--borderStrong);
    pointer-events: none;
  }
  .cross-v {
    left: 50%;
    top: 8%;
    bottom: 8%;
    width: 2px;
    transform: translateX(-50%);
  }
  .cross-h {
    top: 50%;
    left: 8%;
    right: 8%;
    height: 2px;
    transform: translateY(-50%);
  }
  .knob {
    position: absolute;
    width: 22%;
    aspect-ratio: 1;
    border-radius: 50%;
    background: var(--surfaceRaised);
    border: 1px solid var(--borderStrong);
    box-shadow: 0 10px 20px rgba(0, 0, 0, 0.32);
    transform: translate(-50%, -50%);
    transition: left 80ms ease-out, top 80ms ease-out;
    pointer-events: none;
    z-index: 1;
  }
  .pad.active .knob {
    transition: none;
  }
  .labels {
    display: flex;
    gap: 0.5rem;
    font-size: 0.7rem;
    color: var(--textDim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .labels span {
    flex: 1;
    text-align: center;
  }
</style>
