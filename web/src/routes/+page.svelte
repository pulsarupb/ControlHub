<script>
  import { onMount } from 'svelte';

  const UI_RATE_MS = 50;
  const STATUS_RATE_MS = 200;

  let status = null;
  let throttle = 0;
  let steering = 0;
  let joystickActive = false;
  let joystick = { x: 0, y: 0 };
  let joystickEl;
  let mapEl;
  let mapSize = { width: 640, height: 360 };
  let pendingRequest = false;
  let connectionState = 'connecting';

  $: pose = status?.pose ?? { x_m: 0, y_m: 0, heading_rad: 0 };
  $: path = status?.path ?? [];
  $: stopped = status?.emergency_stop || status?.watchdog_stopped;
  $: pathPoints = buildPathPoints(path, mapSize.width, mapSize.height);
  $: robotTransform = robotPoseTransform(pose, mapSize.width, mapSize.height);

  onMount(() => {
    const resizeObserver = new ResizeObserver(([entry]) => {
      mapSize = {
        width: Math.max(320, entry.contentRect.width),
        height: Math.max(220, Math.min(420, entry.contentRect.height - 44))
      };
    });

    if (mapEl) {
      resizeObserver.observe(mapEl);
    }

    refreshStatus();
    const controlTimer = setInterval(sendControlTick, UI_RATE_MS);
    const statusTimer = setInterval(refreshStatus, STATUS_RATE_MS);

    const release = () => releaseJoystick();
    window.addEventListener('blur', release);
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) releaseJoystick();
    });

    return () => {
      clearInterval(controlTimer);
      clearInterval(statusTimer);
      window.removeEventListener('blur', release);
      resizeObserver.disconnect();
      stopRover();
    };
  });

  async function api(path, options = {}) {
    const response = await fetch(path, {
      headers: { 'content-type': 'application/json' },
      ...options
    });

    if (!response.ok) {
      throw new Error(`${path} failed with ${response.status}`);
    }

    status = await response.json();
    connectionState = 'online';
    return status;
  }

  async function refreshStatus() {
    try {
      await api('/api/status');
    } catch (error) {
      connectionState = 'offline';
    }
  }

  async function sendControlTick() {
    if (pendingRequest || status?.emergency_stop) return;

    pendingRequest = true;
    try {
      await api('/api/control', {
        method: 'POST',
        body: JSON.stringify({ throttle, steering })
      });
    } catch (error) {
      connectionState = 'offline';
    } finally {
      pendingRequest = false;
    }
  }

  async function stopRover() {
    throttle = 0;
    steering = 0;
    joystick = { x: 0, y: 0 };
    joystickActive = false;

    try {
      await api('/api/stop', { method: 'POST' });
    } catch (error) {
      connectionState = 'offline';
    }
  }

  async function resetRover() {
    throttle = 0;
    steering = 0;
    joystick = { x: 0, y: 0 };
    joystickActive = false;

    try {
      await api('/api/reset', { method: 'POST' });
    } catch (error) {
      connectionState = 'offline';
    }
  }

  function startJoystick(event) {
    if (status?.emergency_stop) return;
    joystickActive = true;
    joystickEl?.setPointerCapture?.(event.pointerId);
    updateJoystick(event);
  }

  function updateJoystick(event) {
    if (!joystickActive || !joystickEl) return;

    const rect = joystickEl.getBoundingClientRect();
    const radius = Math.min(rect.width, rect.height) * 0.5;
    const centerX = rect.left + rect.width * 0.5;
    const centerY = rect.top + rect.height * 0.5;
    const rawX = (event.clientX - centerX) / radius;
    const rawY = (event.clientY - centerY) / radius;
    const length = Math.hypot(rawX, rawY);
    const scale = length > 1 ? 1 / length : 1;

    joystick = { x: rawX * scale, y: rawY * scale };
    steering = clamp(joystick.x, -1, 1);
    throttle = clamp(-joystick.y, -1, 1);
  }

  function releaseJoystick() {
    joystickActive = false;
    throttle = 0;
    steering = 0;
    joystick = { x: 0, y: 0 };
  }

  function buildPathPoints(points, width, height) {
    if (!points.length) return '';
    const scale = mapScale(points);
    return points.map((point) => `${width / 2 + point.x_m * scale},${height / 2 - point.y_m * scale}`).join(' ');
  }

  function robotPoseTransform(point, width, height) {
    const scale = mapScale(path.length ? path : [point]);
    const x = width / 2 + point.x_m * scale;
    const y = height / 2 - point.y_m * scale;
    const deg = -point.heading_rad * 180 / Math.PI;
    return `translate(${x} ${y}) rotate(${deg})`;
  }

  function mapScale(points) {
    const maxDistance = points.reduce((max, point) => Math.max(max, Math.abs(point.x_m), Math.abs(point.y_m)), 1);
    return Math.min(mapSize.width, mapSize.height) * 0.38 / Math.max(1, maxDistance);
  }

  function clamp(value, min, max) {
    return Math.max(min, Math.min(max, value));
  }
</script>

<svelte:head>
  <title>Rover Control</title>
  <meta name="description" content="Embedded rover control panel" />
</svelte:head>

<main class:stopped>
  <section class="hero">
    <div>
      <p class="eyebrow">Embedded Rover Console</p>
      <h1>Moteus rover control</h1>
      <p class="subtitle">20 Hz joystick commands, 50 Hz motor loop, backend watchdog stop on UI disconnect.</p>
    </div>

    <div class="status-card">
      <span class="status-dot {connectionState}"></span>
      <strong>{connectionState}</strong>
      <span>{stopped ? 'motion locked' : 'ready for motion'}</span>
    </div>
  </section>

  <section class="grid">
    <article class="control-card danger-card">
      <button class="estop" on:click={stopRover} aria-label="Emergency stop all rover motors">
        <span>EMERGENCY</span>
        <strong>STOP</strong>
      </button>
      <button class="reset" on:click={resetRover}>Reset Everything</button>
      <p class="safety-copy">Reset clears the emergency stop, zeroes commands, and resets the pose/path origin.</p>
    </article>

    <article class="control-card joystick-card">
      <div class="card-title">
        <h2>Joystick</h2>
        <span>T {throttle.toFixed(2)} / S {steering.toFixed(2)}</span>
      </div>

      <div
        class="joystick"
        class:active={joystickActive}
        bind:this={joystickEl}
        role="application"
        aria-label="Rover joystick controller"
        on:pointerdown={startJoystick}
        on:pointermove={updateJoystick}
        on:pointerup={releaseJoystick}
        on:pointercancel={releaseJoystick}
        on:pointerleave={releaseJoystick}
      >
        <div class="axis horizontal"></div>
        <div class="axis vertical"></div>
        <div class="stick" style={`transform: translate(${joystick.x * 78}px, ${joystick.y * 78}px)`}></div>
      </div>
      <p>Drag upward to drive forward. Move left or right to steer.</p>
    </article>

    <article class="map-card" bind:this={mapEl}>
      <div class="card-title">
        <h2>Robot Prediction</h2>
        <span>x {pose.x_m.toFixed(2)}m / y {pose.y_m.toFixed(2)}m / h {(pose.heading_rad * 180 / Math.PI).toFixed(0)}deg</span>
      </div>

      <svg viewBox={`0 0 ${mapSize.width} ${mapSize.height}`} preserveAspectRatio="none">
        <defs>
          <pattern id="grid" width="34" height="34" patternUnits="userSpaceOnUse">
            <path d="M 34 0 L 0 0 0 34" fill="none" stroke="rgba(36,48,65,.12)" stroke-width="1" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" rx="24" fill="url(#grid)" />
        <line x1={mapSize.width / 2} y1="0" x2={mapSize.width / 2} y2={mapSize.height} class="origin" />
        <line x1="0" y1={mapSize.height / 2} x2={mapSize.width} y2={mapSize.height / 2} class="origin" />
        <polyline points={pathPoints} class="trail" />
        <g transform={robotTransform} class="robot">
          <path d="M 34 0 L -24 -20 L -14 0 L -24 20 Z" />
          <circle cx="0" cy="0" r="8" />
        </g>
      </svg>
    </article>

    <article class="telemetry-card">
      <div class="card-title">
        <h2>Motor Telemetry</h2>
        <span>{status?.last_error ?? 'nominal'}</span>
      </div>
      <div class="motor-grid">
        {#each status?.motors ?? [] as motor}
          <div class="motor">
            <strong>Motor {motor.id}</strong>
            <span>pos {motor.position.toFixed(3)}</span>
            <span>vel {motor.velocity.toFixed(3)}</span>
            <span>fault {motor.fault}</span>
          </div>
        {/each}
      </div>
    </article>
  </section>
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    min-width: 320px;
    background:
      radial-gradient(circle at 12% 10%, rgba(255, 77, 46, 0.22), transparent 30rem),
      radial-gradient(circle at 85% 20%, rgba(38, 112, 92, 0.2), transparent 28rem),
      linear-gradient(135deg, #eef1e8 0%, #d9e1d1 48%, #b9c7b5 100%);
    color: #162019;
    font-family: Georgia, 'Times New Roman', serif;
  }

  main {
    height: 100vh;
    overflow: hidden;
    padding: clamp(0.7rem, 1.35vw, 1.25rem);
  }

  .hero {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  .eyebrow {
    margin: 0 0 0.35rem;
    color: #8d2d1d;
    font-family: 'Courier New', monospace;
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    max-width: 11ch;
    font-size: clamp(2.4rem, 5.4vw, 5.2rem);
    line-height: 0.86;
    letter-spacing: -0.08em;
  }

  h2 {
    font-size: 1.15rem;
    letter-spacing: -0.04em;
  }

  .subtitle {
    max-width: 40rem;
    margin-top: 0.55rem;
    color: #435147;
    font-family: 'Courier New', monospace;
  }

  .status-card,
  .control-card,
  .map-card,
  .telemetry-card {
    border: 1px solid rgba(22, 32, 25, 0.16);
    border-radius: 22px;
    background: rgba(251, 248, 236, 0.78);
    box-shadow: 0 18px 50px rgba(45, 57, 43, 0.18);
    backdrop-filter: blur(16px);
  }

  .status-card {
    display: grid;
    gap: 0.35rem;
    min-width: 13rem;
    padding: 0.75rem;
    font-family: 'Courier New', monospace;
    text-transform: uppercase;
  }

  .status-dot {
    width: 0.85rem;
    height: 0.85rem;
    border-radius: 999px;
    background: #a9a9a9;
  }

  .status-dot.online {
    background: #1f9b63;
  }

  .status-dot.offline {
    background: #d7261e;
  }

  .grid {
    display: grid;
    grid-template-columns: minmax(240px, 0.8fr) minmax(260px, 0.9fr) minmax(340px, 1.7fr);
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 0.7rem;
    height: calc(100vh - clamp(0.7rem, 1.35vw, 1.25rem) * 2 - 8.4rem);
    min-height: 0;
  }

  .control-card,
  .telemetry-card {
    padding: 0.8rem;
    min-height: 0;
  }

  .danger-card {
    display: grid;
    gap: 0.65rem;
  }

  .estop {
    min-height: clamp(9.5rem, 24vh, 14rem);
    border: 0;
    border-radius: 24px;
    background:
      radial-gradient(circle at 28% 20%, rgba(255, 255, 255, 0.36), transparent 22%),
      linear-gradient(145deg, #ff2518, #8f0802 78%);
    color: white;
    cursor: pointer;
    font-family: Impact, Haettenschweiler, 'Arial Narrow Bold', sans-serif;
    letter-spacing: 0.05em;
    text-shadow: 0 4px 0 rgba(0, 0, 0, 0.22);
    box-shadow: inset 0 -10px 0 rgba(0, 0, 0, 0.18), 0 16px 34px rgba(143, 8, 2, 0.32);
  }

  .estop span,
  .estop strong {
    display: block;
  }

  .estop span {
    font-size: clamp(1.35rem, 3.8vw, 2.45rem);
  }

  .estop strong {
    font-size: clamp(3.2rem, 8.8vw, 6.1rem);
    line-height: 0.8;
  }

  .reset {
    border: 0;
    border-radius: 15px;
    background: #162019;
    color: #fbf8ec;
    cursor: pointer;
    font-family: 'Courier New', monospace;
    font-size: 1rem;
    font-weight: 700;
    padding: 0.75rem;
    text-transform: uppercase;
  }

  .safety-copy,
  .joystick-card p {
    color: #596559;
    font-family: 'Courier New', monospace;
    font-size: 0.9rem;
    line-height: 1.45;
  }

  .joystick-card {
    display: grid;
    gap: 0.7rem;
    align-content: start;
    overflow: hidden;
  }

  .card-title {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.6rem;
    font-family: 'Courier New', monospace;
  }

  .card-title span {
    color: #667060;
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
    border: 1px solid rgba(22, 32, 25, 0.14);
    border-radius: 50%;
    background:
      radial-gradient(circle, rgba(255, 255, 255, 0.84) 0 18%, rgba(226, 232, 214, 0.85) 19% 42%, rgba(154, 171, 145, 0.25) 43% 100%);
    touch-action: none;
  }

  .axis {
    position: absolute;
    background: rgba(22, 32, 25, 0.18);
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
    background: linear-gradient(145deg, #263a2f, #0e1712);
    box-shadow: 0 14px 30px rgba(20, 31, 24, 0.34), inset 0 6px 12px rgba(255, 255, 255, 0.14);
    transition: transform 80ms ease-out;
  }

  .joystick.active .stick {
    transition: none;
  }

  .map-card {
    min-height: 0;
    height: 100%;
    padding: 0.8rem;
    overflow: hidden;
  }

  svg {
    display: block;
    width: 100%;
    height: calc(100% - 2.35rem);
    min-height: 0;
    border-radius: 18px;
    background: rgba(248, 245, 232, 0.82);
  }

  .origin {
    stroke: rgba(22, 32, 25, 0.2);
    stroke-dasharray: 8 8;
  }

  .trail {
    fill: none;
    stroke: #c23a21;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 5;
  }

  .robot path {
    fill: #17251d;
  }

  .robot circle {
    fill: #ff3b20;
  }

  .telemetry-card {
    grid-column: 1 / -1;
    max-height: 9.5rem;
    overflow: hidden;
  }

  .motor-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .motor {
    display: grid;
    gap: 0.35rem;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.46);
    font-family: 'Courier New', monospace;
    padding: 0.65rem;
  }

  .motor span {
    color: #4d5b50;
    font-size: 0.88rem;
  }

  main.stopped .joystick {
    opacity: 0.58;
  }

  @media (max-width: 1100px) {
    .grid {
      grid-template-columns: 1fr 1fr;
      height: auto;
      overflow: visible;
    }

    .map-card,
    .telemetry-card {
      grid-column: 1 / -1;
    }
  }

  @media (max-width: 720px) {
    main {
      height: auto;
      min-height: 100vh;
      overflow: visible;
    }

    .hero,
    .card-title {
      align-items: stretch;
      flex-direction: column;
    }

    .grid,
    .motor-grid {
      grid-template-columns: 1fr;
    }

    .estop {
      min-height: 10rem;
    }

    .subtitle {
      display: none;
    }

    .map-card {
      height: auto;
      min-height: 20rem;
    }

    .telemetry-card {
      max-height: none;
    }
  }
</style>
