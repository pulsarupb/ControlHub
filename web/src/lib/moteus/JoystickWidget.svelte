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
  <header>
    <span>Joystick</span>
    <strong>T {rover.throttle.toFixed(2)} / S {rover.steering.toFixed(2)}</strong>
  </header>

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
  <p>Release, tab away, or lose signal and the backend cuts throttle.</p>
</div>

<style>
  .joystick-widget {
    min-height: 100%;
    display: grid;
    gap: 0.7rem;
    align-content: start;
    overflow: hidden;
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
    text-transform: uppercase;
  }
  header strong {
    color: var(--accent);
    font-size: 0.82rem;
    text-align: right;
  }
  .joystick {
    position: relative;
    display: grid;
    place-items: center;
    width: min(54vw, 240px);
    aspect-ratio: 1;
    margin: 0 auto;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 50%;
    background: radial-gradient(circle, rgba(249, 115, 22, 0.2) 0 18%, rgba(30, 54, 78, 0.92) 19% 42%, rgba(4, 12, 20, 0.82) 43% 100%), conic-gradient(from 45deg, rgba(249, 115, 22, 0.22), rgba(56, 189, 248, 0.18), rgba(249, 115, 22, 0.22));
    box-shadow: inset 0 0 34px rgba(0, 0, 0, 0.68), 0 0 0 3px rgba(255, 255, 255, 0.08);
    touch-action: none;
  }
  .axis {
    position: absolute;
    background: rgba(56, 189, 248, 0.34);
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
    background: linear-gradient(145deg, #facc15, #f97316);
    box-shadow: 0 14px 30px rgba(0, 0, 0, 0.44), inset 0 6px 12px rgba(255, 255, 255, 0.34), 0 0 26px rgba(249, 115, 22, 0.26);
    transition: transform 80ms ease-out;
  }
  .joystick.active .stick {
    transition: none;
  }
  p {
    margin: 0;
    color: rgba(255, 255, 255, 0.72);
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
    font-size: 0.9rem;
    line-height: 1.45;
  }
</style>
