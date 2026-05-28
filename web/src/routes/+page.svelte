<script>
  import { onMount } from "svelte"

  const UI_RATE_MS = 50
  const STATUS_RATE_MS = 200
  const ROVER_LENGTH_M = 1.5
  const ROVER_WIDTH_M = 0.5

  let status = null
  let throttle = 0
  let steering = 0
  let joystickActive = false
  let joystick = { x: 0, y: 0 }
  let joystickEl
  let mapEl
  let mapSize = { width: 640, height: 360 }
  let pendingRequest = false
  let connectionState = "connecting"

  $: pose = status?.pose ?? { x_m: 0, y_m: 0, heading_rad: 0 }
  $: path = status?.path ?? []
  $: stopped = status?.emergency_stop || status?.watchdog_stopped
  $: currentMapScale = mapScale(path.length ? path : [pose])
  $: pathPoints = buildPathPoints(path, mapSize.width, mapSize.height)
  $: robotTransform = robotPoseTransform(pose, mapSize.width, mapSize.height)
  $: roverLengthPx = ROVER_LENGTH_M * currentMapScale
  $: roverWidthPx = ROVER_WIDTH_M * currentMapScale
  $: roverBodyPath = `M ${roverLengthPx / 2} 0 L ${roverLengthPx * 0.25} ${-roverWidthPx / 2} L ${-roverLengthPx / 2} ${-roverWidthPx / 2} L ${-roverLengthPx / 2} ${roverWidthPx / 2} L ${roverLengthPx * 0.25} ${roverWidthPx / 2} Z`

  onMount(() => {
    const resizeObserver = new ResizeObserver(([entry]) => {
      mapSize = {
        width: Math.max(320, entry.contentRect.width),
        height: Math.max(220, Math.min(420, entry.contentRect.height - 44)),
      }
    })

    if (mapEl) {
      resizeObserver.observe(mapEl)
    }

    refreshStatus()
    const controlTimer = setInterval(sendControlTick, UI_RATE_MS)
    const statusTimer = setInterval(refreshStatus, STATUS_RATE_MS)

    const release = () => releaseJoystick()
    window.addEventListener("blur", release)
    document.addEventListener("visibilitychange", () => {
      if (document.hidden) releaseJoystick()
    })

    return () => {
      clearInterval(controlTimer)
      clearInterval(statusTimer)
      window.removeEventListener("blur", release)
      resizeObserver.disconnect()
      stopRover()
    }
  })

  async function api(path, options = {}) {
    const response = await fetch(path, {
      headers: { "content-type": "application/json" },
      ...options,
    })

    if (!response.ok) {
      throw new Error(`${path} failed with ${response.status}`)
    }

    status = await response.json()
    connectionState = "online"
    return status
  }

  async function refreshStatus() {
    try {
      await api("/api/status")
    } catch (error) {
      connectionState = "offline"
    }
  }

  async function sendControlTick() {
    if (pendingRequest || status?.emergency_stop) return

    pendingRequest = true
    try {
      await api("/api/control", {
        method: "POST",
        body: JSON.stringify({ throttle, steering }),
      })
    } catch (error) {
      connectionState = "offline"
    } finally {
      pendingRequest = false
    }
  }

  async function stopRover() {
    throttle = 0
    steering = 0
    joystick = { x: 0, y: 0 }
    joystickActive = false

    try {
      await api("/api/stop", { method: "POST" })
    } catch (error) {
      connectionState = "offline"
    }
  }

  async function resetRover() {
    throttle = 0
    steering = 0
    joystick = { x: 0, y: 0 }
    joystickActive = false

    try {
      await api("/api/reset", { method: "POST" })
    } catch (error) {
      connectionState = "offline"
    }
  }

  function startJoystick(event) {
    if (status?.emergency_stop) return
    joystickActive = true
    joystickEl?.setPointerCapture?.(event.pointerId)
    updateJoystick(event)
  }

  function updateJoystick(event) {
    if (!joystickActive || !joystickEl) return

    const rect = joystickEl.getBoundingClientRect()
    const radius = Math.min(rect.width, rect.height) * 0.5
    const centerX = rect.left + rect.width * 0.5
    const centerY = rect.top + rect.height * 0.5
    const rawX = (event.clientX - centerX) / radius
    const rawY = (event.clientY - centerY) / radius
    const length = Math.hypot(rawX, rawY)
    const scale = length > 1 ? 1 / length : 1

    joystick = { x: rawX * scale, y: rawY * scale }
    steering = clamp(joystick.x, -1, 1)
    throttle = clamp(-joystick.y, -1, 1)
  }

  function releaseJoystick() {
    joystickActive = false
    throttle = 0
    steering = 0
    joystick = { x: 0, y: 0 }
  }

  function buildPathPoints(points, width, height) {
    if (!points.length) return ""
    const scale = mapScale(points)
    return points
      .map(
        (point) =>
          `${width / 2 + point.x_m * scale},${height / 2 - point.y_m * scale}`,
      )
      .join(" ")
  }

  function robotPoseTransform(point, width, height) {
    const x = width / 2 + point.x_m * currentMapScale
    const y = height / 2 - point.y_m * currentMapScale
    const deg = (-point.heading_rad * 180) / Math.PI
    return `translate(${x} ${y}) rotate(${deg})`
  }

  function mapScale(points) {
    const footprintRadius = Math.hypot(ROVER_LENGTH_M, ROVER_WIDTH_M) * 0.5
    const maxDistance = points.reduce(
      (max, point) => Math.max(max, Math.abs(point.x_m), Math.abs(point.y_m)),
      footprintRadius,
    )
    return (
      (Math.min(mapSize.width, mapSize.height) * 0.38) /
      Math.max(1, maxDistance)
    )
  }

  function clamp(value, min, max) {
    return Math.max(min, Math.min(max, value))
  }
</script>

<svelte:head>
  <title>Rover Control</title>
  <meta name="description" content="Embedded rover control panel" />
</svelte:head>

<main class:stopped>
  <section class="hero">
    <div class="status-card">
      <div class="wanted" aria-label="Rover alert level">
        <span class:lit={stopped}>*</span>
        <span class:lit={connectionState === "offline"}>*</span>
        <span>*</span>
        <span>*</span>
        <span>*</span>
      </div>
      <span class="status-dot {connectionState}"></span>
      <strong>{connectionState}</strong>
      <span>{stopped ? "motion locked" : "ready for motion"}</span>
    </div>
  </section>

  <section class="grid">
    <article class="control-card danger-card">
      <button
        class="estop"
        on:click={stopRover}
        aria-label="Emergency stop all rover motors"
      >
        <span>WASTED?</span>
        <strong>STOP</strong>
      </button>
      <button class="reset" on:click={resetRover}>Reset Everything</button>
      <p class="safety-copy">
        Reset clears the stop, zeroes commands, and drops a fresh minimap spawn
        point.
      </p>
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
        <div
          class="stick"
          style={`transform: translate(${joystick.x * 78}px, ${joystick.y * 78}px)`}
        ></div>
      </div>
      <p>
        Left-stick style control. Release, tab away, or lose signal and the
        backend cuts throttle.
      </p>
    </article>

    <article class="map-card" bind:this={mapEl}>
      <div class="card-title">
        <h2>Minimap Prediction</h2>
        <span>
          x {pose.x_m.toFixed(2)}m / y {pose.y_m.toFixed(2)}m / h {(
            (pose.heading_rad * 180) /
            Math.PI
          ).toFixed(0)}deg
        </span>
      </div>

      <svg
        viewBox={`0 0 ${mapSize.width} ${mapSize.height}`}
        preserveAspectRatio="none"
      >
        <defs>
          <pattern
            id="grid"
            width="34"
            height="34"
            patternUnits="userSpaceOnUse"
          >
            <path
              d="M 34 0 L 0 0 0 34"
              fill="none"
              stroke="rgba(36,48,65,.12)"
              stroke-width="1"
            />
          </pattern>
        </defs>
        <circle
          cx={mapSize.width / 2}
          cy={mapSize.height / 2}
          r={Math.min(mapSize.width, mapSize.height) * 0.48}
          fill="url(#grid)"
        />
        <line
          x1={mapSize.width / 2}
          y1="0"
          x2={mapSize.width / 2}
          y2={mapSize.height}
          class="origin"
        />
        <line
          x1="0"
          y1={mapSize.height / 2}
          x2={mapSize.width}
          y2={mapSize.height / 2}
          class="origin"
        />
        <polyline points={pathPoints} class="trail" />
        <g transform={robotTransform} class="robot">
          <path d={roverBodyPath} />
          <rect
            x={-roverLengthPx * 0.34}
            y={-roverWidthPx * 0.66}
            width={roverLengthPx * 0.22}
            height={roverWidthPx * 0.22}
            rx="3"
          />
          <rect
            x={-roverLengthPx * 0.34}
            y={roverWidthPx * 0.44}
            width={roverLengthPx * 0.22}
            height={roverWidthPx * 0.22}
            rx="3"
          />
          <rect
            x={roverLengthPx * 0.18}
            y={-roverWidthPx * 0.66}
            width={roverLengthPx * 0.22}
            height={roverWidthPx * 0.22}
            rx="3"
          />
          <rect
            x={roverLengthPx * 0.18}
            y={roverWidthPx * 0.44}
            width={roverLengthPx * 0.22}
            height={roverWidthPx * 0.22}
            rx="3"
          />
          <circle cx={roverLengthPx * 0.22} cy="0" r={Math.max(2, roverWidthPx * 0.08)} />
        </g>
      </svg>
    </article>

    <article class="telemetry-card">
      <div class="card-title">
        <h2>Garage Telemetry</h2>
        <span>{status?.last_error ?? "nominal"}</span>
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
    background: linear-gradient(rgba(255, 255, 255, 0.035) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255, 255, 255, 0.035) 1px, transparent 1px),
      radial-gradient(
        circle at 16% 24%,
        rgba(255, 188, 64, 0.26),
        transparent 22rem
      ),
      radial-gradient(
        circle at 86% 12%,
        rgba(52, 202, 255, 0.22),
        transparent 24rem
      ),
      radial-gradient(
        circle at 74% 88%,
        rgba(255, 54, 96, 0.26),
        transparent 24rem
      ),
      linear-gradient(145deg, #071019 0%, #10283a 48%, #090b12 100%);
    background-size:
      34px 34px,
      34px 34px,
      auto,
      auto,
      auto,
      auto;
    color: #f5f2dc;
    font-family: Impact, Haettenschweiler, "Arial Narrow Bold", sans-serif;
    overflow: hidden;
  }

  :global(body::before) {
    content: "";
    position: fixed;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(
      180deg,
      transparent 0 50%,
      rgba(0, 0, 0, 0.18) 51% 100%
    );
    background-size: 100% 4px;
    mix-blend-mode: multiply;
    opacity: 0.45;
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
    color: #ffcf4f;
    font-family: "Courier New", monospace;
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
    max-width: 13ch;
    font-size: clamp(2.4rem, 5.4vw, 5.2rem);
    line-height: 0.86;
    letter-spacing: -0.045em;
    text-shadow:
      0 5px 0 #041018,
      0 0 24px rgba(255, 207, 79, 0.2);
    text-transform: uppercase;
  }

  h2 {
    font-size: 1.15rem;
    letter-spacing: -0.04em;
  }

  .subtitle {
    max-width: 40rem;
    margin-top: 0.55rem;
    color: #b9d3d0;
    font-family: "Courier New", monospace;
  }

  .status-card,
  .control-card,
  .map-card,
  .telemetry-card {
    border: 2px solid rgba(255, 255, 255, 0.28);
    border-radius: 8px;
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.12),
        rgba(255, 255, 255, 0.04)
      ),
      rgba(8, 18, 28, 0.78);
    box-shadow:
      0 18px 48px rgba(0, 0, 0, 0.42),
      inset 0 0 0 1px rgba(0, 0, 0, 0.42);
    backdrop-filter: blur(10px) saturate(1.2);
  }

  .status-card {
    display: grid;
    gap: 0.35rem;
    min-width: 13rem;
    padding: 0.75rem;
    font-family: "Courier New", monospace;
    text-transform: uppercase;
    box-shadow:
      0 0 0 3px rgba(0, 0, 0, 0.22),
      0 14px 36px rgba(0, 0, 0, 0.32);
  }

  .wanted {
    display: flex;
    gap: 0.25rem;
    color: rgba(255, 255, 255, 0.28);
    font-family: Georgia, "Times New Roman", serif;
    font-size: 1.4rem;
    line-height: 1;
  }

  .wanted .lit {
    color: #ffcf4f;
    text-shadow: 0 0 10px rgba(255, 207, 79, 0.9);
  }

  .status-dot {
    width: 0.85rem;
    height: 0.85rem;
    border-radius: 999px;
    background: #a9a9a9;
  }

  .status-dot.online {
    background: #39ff8f;
    box-shadow: 0 0 14px rgba(57, 255, 143, 0.86);
  }

  .status-dot.offline {
    background: #ff2136;
    box-shadow: 0 0 14px rgba(255, 33, 54, 0.86);
  }

  .grid {
    display: grid;
    grid-template-columns: minmax(240px, 0.8fr) minmax(260px, 0.9fr) minmax(
        340px,
        1.7fr
      );
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
    border-radius: 12px;
    background:
      linear-gradient(90deg, rgba(255, 255, 255, 0.08) 50%, transparent 50%) 0 0 /
        18px 18px,
      radial-gradient(
        circle at 28% 20%,
        rgba(255, 255, 255, 0.38),
        transparent 22%
      ),
      linear-gradient(145deg, #ff2d18, #8c0308 78%);
    color: white;
    cursor: pointer;
    font-family: Impact, Haettenschweiler, "Arial Narrow Bold", sans-serif;
    letter-spacing: 0.05em;
    text-shadow:
      0 5px 0 rgba(0, 0, 0, 0.42),
      0 0 18px rgba(255, 255, 255, 0.2);
    box-shadow:
      inset 0 -10px 0 rgba(0, 0, 0, 0.26),
      0 0 0 4px rgba(255, 255, 255, 0.16),
      0 18px 40px rgba(255, 33, 54, 0.38);
    transform: skew(-2deg);
  }

  .estop:hover {
    filter: saturate(1.18) brightness(1.06);
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
    border: 2px solid rgba(255, 207, 79, 0.8);
    border-radius: 8px;
    background: linear-gradient(180deg, #ffe36c, #d09318);
    color: #161008;
    cursor: pointer;
    font-family: "Courier New", monospace;
    font-size: 1rem;
    font-weight: 700;
    padding: 0.75rem;
    text-transform: uppercase;
    box-shadow:
      inset 0 -4px 0 rgba(0, 0, 0, 0.22),
      0 0 24px rgba(255, 207, 79, 0.22);
  }

  .safety-copy,
  .joystick-card p {
    color: #596559;
    color: #b9d3d0;
    font-family: "Courier New", monospace;
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
    font-family: "Courier New", monospace;
    text-transform: uppercase;
  }

  .card-title span {
    color: #8fe8ff;
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
    background: radial-gradient(
        circle,
        rgba(255, 207, 79, 0.22) 0 18%,
        rgba(30, 54, 78, 0.92) 19% 42%,
        rgba(4, 12, 20, 0.82) 43% 100%
      ),
      conic-gradient(
        from 45deg,
        rgba(255, 207, 79, 0.22),
        rgba(92, 216, 255, 0.18),
        rgba(255, 207, 79, 0.22)
      );
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
    border: 2px solid rgba(255, 255, 255, 0.22);
    border-radius: 50%;
    background: radial-gradient(
      circle,
      rgba(28, 66, 74, 0.92),
      rgba(7, 17, 22, 0.94)
    );
    box-shadow:
      inset 0 0 42px rgba(0, 0, 0, 0.74),
      0 0 0 5px rgba(0, 0, 0, 0.26);
  }

  .origin {
    stroke: rgba(143, 232, 255, 0.22);
    stroke-dasharray: 7 9;
  }

  .trail {
    fill: none;
    stroke: #ffcf4f;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 5;
  }

  .robot path {
    fill: #f5f2dc;
    filter: drop-shadow(0 0 8px rgba(255, 207, 79, 0.85));
  }

  .robot rect {
    fill: #08121c;
    stroke: #8fe8ff;
    stroke-width: 1.5;
  }

  .robot circle {
    fill: #2dff8c;
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
    border: 1px solid rgba(143, 232, 255, 0.24);
    border-radius: 8px;
    background: rgba(4, 12, 20, 0.48);
    font-family: "Courier New", monospace;
    padding: 0.65rem;
  }

  .motor span {
    color: #b9d3d0;
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
