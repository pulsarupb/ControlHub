<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"

  let joystickEl: HTMLDivElement | undefined = $state()

  function pointerPosition(event: PointerEvent): void {
    if (!joystickEl) return

    const rect = joystickEl.getBoundingClientRect()
    const radius = Math.min(rect.width, rect.height) * 0.5
    const centerX = rect.left + rect.width * 0.5
    const centerY = rect.top + rect.height * 0.5

    rover.setJoystick((event.clientX - centerX) / radius, (event.clientY - centerY) / radius)
  }

  function handlePointerDown(event: PointerEvent): void {
    if (!rover.startJoystick()) return

    joystickEl?.setPointerCapture?.(event.pointerId)
    pointerPosition(event)
  }
</script>

<div class="joystick-widget">
  <strong class="readout">T {rover.throttle.toFixed(2)} / S {rover.steering.toFixed(2)}</strong>

  <div
    class="joystick"
    class:active={rover.joystickActive}
    bind:this={joystickEl}
    role="application"
    aria-label="Rover joystick controller"
    onpointerdown={handlePointerDown}
    onpointermove={pointerPosition}
    onpointerup={rover.releaseJoystick}
    onpointercancel={rover.releaseJoystick}
    onpointerleave={rover.releaseJoystick}
  >
    <div class="axis horizontal"></div>
    <div class="axis vertical"></div>
    <div class="stick" style={`transform: translate(${rover.joystick.x * 78}px, ${rover.joystick.y * 78}px)`}></div>
  </div>
</div>

<style>
  .joystick-widget {
    min-height: 100%;
    display: grid;
    gap: 0.7rem;
    align-content: start;
    overflow: hidden;
  }
  .readout {
    color: var(--accent);
    font-size: 0.82rem;
    text-align: right;
    text-transform: uppercase;
  }
  .joystick {
    position: relative;
    display: grid;
    place-items: center;
    width: min(54vw, 240px);
    aspect-ratio: 1;
    margin: 0 auto;
    border: 1px solid var(--borderStrong);
    border-radius: 50%;
    background: var(--bgDark);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.03);
    touch-action: none;
  }
  .axis {
    position: absolute;
    background: var(--borderStrong);
  }
  .axis.horizontal {
    width: 76%;
    height: 2px;
  }
  .axis.vertical {
    width: 2px;
    height: 76%;
  }
  .stick {
    width: 4.7rem;
    aspect-ratio: 1;
    border-radius: 50%;
    background: var(--surfaceRaised);
    border: 1px solid var(--borderStrong);
    box-shadow: 0 10px 20px rgba(0, 0, 0, 0.32);
    transition: transform 80ms ease-out;
  }
  .joystick.active .stick {
    transition: none;
  }
</style>
