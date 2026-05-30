<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"

  const PAD_RADIUS_M = 1

  let padEl: HTMLDivElement | undefined = $state()
  let selected = $state({ x_m: 0.5, y_m: 0 })

  const selectedDistance = $derived(Math.hypot(selected.x_m, selected.y_m))
  const canFollow = $derived(!rover.stopped && rover.connectionState === "online" && selectedDistance > 0.02)
  const markerLeft = $derived(50 + (selected.y_m / PAD_RADIUS_M) * 50)
  const markerTop = $derived(50 - (selected.x_m / PAD_RADIUS_M) * 50)
  const activeTarget = $derived(rover.follower.target)

  function selectTarget(event: PointerEvent): void {
    if (!padEl || rover.followerActive) return

    event.preventDefault()

    const rect = padEl.getBoundingClientRect()
    const radiusPx = Math.min(rect.width, rect.height) * 0.5
    const centerX = rect.left + rect.width * 0.5
    const centerY = rect.top + rect.height * 0.5
    const rawY = ((event.clientX - centerX) / radiusPx) * PAD_RADIUS_M
    const rawX = (-(event.clientY - centerY) / radiusPx) * PAD_RADIUS_M
    const distance = Math.hypot(rawX, rawY)
    const scale = distance > PAD_RADIUS_M ? PAD_RADIUS_M / distance : 1

    selected = {
      x_m: rawX * scale,
      y_m: rawY * scale,
    }
  }

  function followSelected(): void {
    if (!canFollow) return
    rover.startFollowerTarget(selected.x_m, selected.y_m)
  }
</script>

<div class="target-follower-widget">
  <header>
    <span>Target follower</span>
    <strong>{rover.followerActive ? "FOLLOWING" : "READY"}</strong>
  </header>

  <div
    class="target-pad"
    class:locked={rover.followerActive}
    bind:this={padEl}
    role="application"
    aria-label="Select a rover-relative target within one meter"
    onpointerdown={selectTarget}
    onpointermove={(event) => event.buttons === 1 && selectTarget(event)}
  >
    <div class="ring half"></div>
    <div class="ring full"></div>
    <div class="axis forward">FWD</div>
    <div class="axis lateral">LEFT / RIGHT</div>
    <div class="rover-dot"></div>
    <div class="target-line" style={`width: ${selectedDistance * 50}%; transform: rotate(${Math.atan2(-selected.x_m, selected.y_m)}rad)`}></div>
    <div class="target-marker" style={`left: ${markerLeft}%; top: ${markerTop}%`}></div>
  </div>

  <div class="readout">
    <span>Target x {selected.x_m.toFixed(2)}m</span>
    <span>y {selected.y_m.toFixed(2)}m</span>
    <span>r {selectedDistance.toFixed(2)}m</span>
  </div>

  {#if rover.followerActive}
    <p class="status">Follower active: {rover.follower.distance_m.toFixed(2)}m remaining.</p>
  {:else if activeTarget && rover.follower.arrived}
    <p class="status">Target reached. Manual joystick is available.</p>
  {:else}
    <p class="status">Pick a point inside the 1m circle. Joystick input is rejected while following.</p>
  {/if}

  <div class="actions">
    <button type="button" onclick={followSelected} disabled={!canFollow || rover.followerActive}>Follow</button>
    <button type="button" class="secondary" onclick={rover.cancelFollowerTarget} disabled={!rover.followerActive}>Cancel</button>
  </div>
</div>

<style>
  .target-follower-widget {
    min-height: 100%;
    display: grid;
    gap: 0.65rem;
    align-content: start;
    overflow: hidden;
    user-select: none;
  }
  header,
  .readout,
  .status,
  button {
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    text-transform: uppercase;
  }
  header strong {
    color: var(--accent);
    font-size: 0.82rem;
  }
  .target-pad {
    position: relative;
    width: min(54vw, 250px);
    aspect-ratio: 1;
    margin: 0 auto;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 50%;
    background: radial-gradient(circle, rgba(45, 212, 191, 0.16) 0 17%, rgba(18, 45, 53, 0.95) 18% 57%, rgba(5, 15, 21, 0.92) 58% 100%), conic-gradient(from 0deg, rgba(45, 212, 191, 0.18), rgba(250, 204, 21, 0.2), rgba(45, 212, 191, 0.18));
    box-shadow: inset 0 0 38px rgba(0, 0, 0, 0.7), 0 0 0 4px rgba(255, 255, 255, 0.07);
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
  }
  .target-pad.locked {
    cursor: not-allowed;
    opacity: 0.76;
  }
  .ring {
    position: absolute;
    inset: 25%;
    border: 1px dashed rgba(255, 255, 255, 0.22);
    border-radius: 50%;
  }
  .ring.full {
    inset: 7%;
  }
  .axis {
    position: absolute;
    color: rgba(255, 255, 255, 0.42);
    font-size: 0.7rem;
    letter-spacing: 0.08em;
    pointer-events: none;
  }
  .axis.forward {
    top: 0.75rem;
    left: 50%;
    transform: translateX(-50%);
  }
  .axis.lateral {
    left: 50%;
    top: 50%;
    width: 78%;
    text-align: center;
    transform: translate(-50%, -50%);
  }
  .rover-dot,
  .target-marker {
    position: absolute;
    border-radius: 50%;
    transform: translate(-50%, -50%);
  }
  .rover-dot {
    left: 50%;
    top: 50%;
    width: 1.25rem;
    aspect-ratio: 1;
    background: #fff7ed;
    box-shadow: 0 0 16px rgba(255, 247, 237, 0.72);
  }
  .target-marker {
    width: 1.05rem;
    aspect-ratio: 1;
    background: #facc15;
    box-shadow: 0 0 18px rgba(250, 204, 21, 0.86);
  }
  .target-line {
    position: absolute;
    left: 50%;
    top: 50%;
    height: 2px;
    transform-origin: left center;
    background: linear-gradient(90deg, rgba(250, 204, 21, 0.9), rgba(250, 204, 21, 0));
  }
  .readout {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.45rem;
    color: var(--accent);
    font-size: 0.84rem;
  }
  .status {
    margin: 0;
    color: rgba(255, 255, 255, 0.72);
    font-size: 0.86rem;
    line-height: 1.4;
    text-align: center;
  }
  .actions {
    display: flex;
    justify-content: center;
    gap: 0.6rem;
  }
  button {
    border: 1px solid rgba(250, 204, 21, 0.55);
    border-radius: 999rem;
    padding: 0.45rem 0.9rem;
    background: linear-gradient(135deg, #facc15, #f97316);
    color: #071117;
    font-weight: 800;
    cursor: pointer;
  }
  button.secondary {
    border-color: rgba(255, 255, 255, 0.22);
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.86);
  }
  button:disabled {
    cursor: not-allowed;
    filter: grayscale(0.75);
    opacity: 0.45;
  }
</style>
