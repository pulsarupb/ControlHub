<script lang="ts">
  import CardTitle from "$lib/components/CardTitle.svelte";
  import type { JoystickPosition } from "$lib/types";

  type Props = {
    throttle: number;
    steering: number;
    joystick: JoystickPosition;
    joystickActive: boolean;
    startJoystick: () => boolean;
    updateJoystick: (rawX: number, rawY: number) => void;
    releaseJoystick: () => void;
  };

  let { throttle, steering, joystick, joystickActive, startJoystick, updateJoystick, releaseJoystick }: Props = $props();
  let joystickEl: HTMLDivElement | undefined = $state();

  function pointerPosition(event: PointerEvent): void {
    if (!joystickEl) return;

    const rect = joystickEl.getBoundingClientRect();
    const radius = Math.min(rect.width, rect.height) * 0.5;
    const centerX = rect.left + rect.width * 0.5;
    const centerY = rect.top + rect.height * 0.5;

    updateJoystick((event.clientX - centerX) / radius, (event.clientY - centerY) / radius);
  }

  function handlePointerDown(event: PointerEvent): void {
    if (!startJoystick()) return;

    joystickEl?.setPointerCapture?.(event.pointerId);
    pointerPosition(event);
  }
</script>

<article class="control-card joystick-card">
  <CardTitle title="Joystick" detail={`T ${throttle.toFixed(2)} / S ${steering.toFixed(2)}`} />

  <div
    class="joystick"
    class:active={joystickActive}
    bind:this={joystickEl}
    role="application"
    aria-label="Rover joystick controller"
    onpointerdown={handlePointerDown}
    onpointermove={pointerPosition}
    onpointerup={releaseJoystick}
    onpointercancel={releaseJoystick}
    onpointerleave={releaseJoystick}
  >
    <div class="axis horizontal"></div>
    <div class="axis vertical"></div>
    <div class="stick" style={`transform: translate(${joystick.x * 78}px, ${joystick.y * 78}px)`}></div>
  </div>
  <p>Left-stick style control. Release, tab away, or lose signal and the backend cuts throttle.</p>
</article>

<style>
  .control-card {
    min-height: 0;
    border: 2px solid rgba(255, 255, 255, 0.28);
    border-radius: 8px;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.04)), rgba(8, 18, 28, 0.78);
    box-shadow:
      0 18px 48px rgba(0, 0, 0, 0.42),
      inset 0 0 0 1px rgba(0, 0, 0, 0.42);
    padding: 0.8rem;
    backdrop-filter: blur(10px) saturate(1.2);
  }

  .joystick-card {
    display: grid;
    gap: 0.7rem;
    align-content: start;
    overflow: hidden;
  }

  .joystick {
    position: relative;
    display: grid;
    place-items: center;
    width: min(54vw, 240px);
    aspect-ratio: 1;
    margin: 0 auto;
    border: 1px solid rgba(22, 32, 25, 0.14);
    border-radius: 50%;
    background: radial-gradient(circle, rgba(255, 207, 79, 0.22) 0 18%, rgba(30, 54, 78, 0.92) 19% 42%, rgba(4, 12, 20, 0.82) 43% 100%),
      conic-gradient(from 45deg, rgba(255, 207, 79, 0.22), rgba(92, 216, 255, 0.18), rgba(255, 207, 79, 0.22));
    box-shadow:
      inset 0 0 34px rgba(0, 0, 0, 0.68),
      0 0 0 3px rgba(255, 255, 255, 0.14);
    touch-action: none;
  }

  .axis {
    position: absolute;
    background: rgba(143, 232, 255, 0.34);
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
    background: linear-gradient(145deg, #f9e59d, #9c5b0d);
    box-shadow:
      0 14px 30px rgba(0, 0, 0, 0.44),
      inset 0 6px 12px rgba(255, 255, 255, 0.34),
      0 0 26px rgba(255, 207, 79, 0.26);
    transition: transform 80ms ease-out;
  }

  .joystick.active .stick {
    transition: none;
  }

  p {
    margin: 0;
    color: #b9d3d0;
    font-family: "Courier New", monospace;
    font-size: 0.9rem;
    line-height: 1.45;
  }
</style>
