<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"

  let fwdEl: HTMLDivElement | undefined = $state()
  let spinEl: HTMLDivElement | undefined = $state()
  let fwdActive = $state(false)
  let spinActive = $state(false)

  const FWD_RANGE = 65
  const SPIN_RANGE = 40

  function fwdPointerPos(event: PointerEvent): void {
    if (!fwdActive || !fwdEl) return
    const rect = fwdEl.getBoundingClientRect()
    const halfHeight = rect.height * 0.5
    const centerY = rect.top + halfHeight
    const rawY = (event.clientY - centerY) / halfHeight
    rover.setThrottle(rawY)
  }

  function spinPointerPos(event: PointerEvent): void {
    if (!spinActive || !spinEl) return
    const rect = spinEl.getBoundingClientRect()
    const halfWidth = rect.width * 0.5
    const centerX = rect.left + halfWidth
    const rawX = (event.clientX - centerX) / halfWidth
    rover.setSteering(rawX)
  }

  function handleFwdDown(event: PointerEvent): void {
    if (!rover.startJoystick()) return
    fwdEl?.setPointerCapture(event.pointerId)
    fwdActive = true
    fwdPointerPos(event)
  }

  function handleSpinDown(event: PointerEvent): void {
    if (!rover.startJoystick()) return
    spinEl?.setPointerCapture(event.pointerId)
    spinActive = true
    spinPointerPos(event)
  }

  function releaseFwd(): void {
    fwdActive = false
    rover.setThrottle(0)
    if (!fwdActive && !spinActive) rover.releaseJoystick()
  }

  function releaseSpin(): void {
    spinActive = false
    rover.setSteering(0)
    if (!fwdActive && !spinActive) rover.releaseJoystick()
  }
</script>

<div class="joystick-widget">
  <strong class="readout">T {rover.throttle.toFixed(2)} / S {rover.steering.toFixed(2)}</strong>

  <div class="sliders">
    <div
      class="slider-area vertical"
      class:active={fwdActive}
      bind:this={fwdEl}
      role="slider"
      tabindex="0"
      aria-label="Forward and backward"
      aria-valuemin={-1}
      aria-valuemax={1}
      aria-valuenow={rover.throttle}
      onpointerdown={handleFwdDown}
      onpointermove={fwdPointerPos}
      onpointerup={releaseFwd}
      onpointercancel={releaseFwd}
      onpointerleave={releaseFwd}
    >
      <div class="track"></div>
      <div class="puck" style={`transform: translateY(${rover.throttle * FWD_RANGE}px)`}></div>
    </div>

    <div
      class="slider-area horizontal"
      class:active={spinActive}
      bind:this={spinEl}
      role="slider"
      tabindex="0"
      aria-label="Spin left and right"
      aria-valuemin={-1}
      aria-valuemax={1}
      aria-valuenow={rover.steering}
      onpointerdown={handleSpinDown}
      onpointermove={spinPointerPos}
      onpointerup={releaseSpin}
      onpointercancel={releaseSpin}
      onpointerleave={releaseSpin}
    >
      <div class="track"></div>
      <div class="puck" style={`transform: translateX(${rover.steering * SPIN_RANGE}px)`}></div>
    </div>
  </div>

  <div class="labels">
    <span>Forward / Back</span>
    <span>Spin</span>
  </div>
</div>

<style>
  .joystick-widget {
    min-height: 100%;
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 0.5rem;
    overflow: hidden;
  }
  .readout {
    color: var(--accent);
    font-size: 0.82rem;
    text-align: right;
    text-transform: uppercase;
  }
  .sliders {
    display: flex;
    gap: 0.5rem;
  }
  .slider-area {
    position: relative;
    flex: 1;
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
  .labels {
    display: flex;
    justify-content: space-around;
    font-size: 0.7rem;
    color: var(--textDim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
</style>
