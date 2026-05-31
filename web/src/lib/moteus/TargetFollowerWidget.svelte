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
  {#if rover.followerActive}
    <strong class="state">FOLLOWING</strong>
  {/if}

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
  .state,
  .readout,
  .status,
  button {
    font-variant-numeric: tabular-nums;
  }
  .state {
    color: var(--accent);
    font-size: 0.82rem;
    text-align: right;
    text-transform: uppercase;
  }
  .target-pad {
    position: relative;
    width: min(54vw, 250px);
    aspect-ratio: 1;
    margin: 0 auto;
    border: 1px solid var(--borderStrong);
    border-radius: 50%;
    background: var(--bgDark);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.03);
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
    border: 1px dashed var(--borderStrong);
    border-radius: 50%;
  }
  .ring.full {
    inset: 7%;
  }
  .axis {
    position: absolute;
    color: var(--textSubtle);
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
    background: var(--text);
  }
  .target-marker {
    width: 1.05rem;
    aspect-ratio: 1;
    background: var(--warning);
    border: 1px solid rgba(0, 0, 0, 0.32);
  }
  .target-line {
    position: absolute;
    left: 50%;
    top: 50%;
    height: 2px;
    transform-origin: left center;
    background: var(--warning);
  }
  .readout {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.45rem;
    color: var(--textMuted);
    font-size: 0.84rem;
  }
  .status {
    margin: 0;
    color: var(--textMuted);
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
    border: 1px solid var(--borderStrong);
    border-radius: 0.4rem;
    padding: 0.45rem 0.9rem;
    background: var(--surfaceRaised);
    color: var(--text);
    font-weight: 700;
    cursor: pointer;
  }
  button.secondary {
    border-color: var(--border);
    background: transparent;
    color: var(--textMuted);
  }
  button:disabled {
    cursor: not-allowed;
    filter: grayscale(0.75);
    opacity: 0.45;
  }
</style>
