<script lang="ts">
  import { ROVER_LENGTH_M, ROVER_WIDTH_M } from "$lib/constants"
  import { buildPathPoints, mapScale, robotPoseTransform, roverBodyPath } from "$lib/map"
  import { radiansToDegrees } from "$lib/math"
  import { rover } from "$lib/rover-control.svelte"
  import type { MapSize } from "$lib/moteus-types"

  let mapEl: HTMLElement | undefined = $state()
  let mapSize: MapSize = $state({ width: 640, height: 360 })

  const currentMapScale = $derived(mapScale(rover.path.length ? rover.path : [rover.pose], mapSize))
  const pathPoints = $derived(buildPathPoints(rover.path, mapSize))
  const robotTransform = $derived(robotPoseTransform(rover.pose, mapSize, currentMapScale))
  const roverLengthPx = $derived(ROVER_LENGTH_M * currentMapScale)
  const roverWidthPx = $derived(ROVER_WIDTH_M * currentMapScale)
  const bodyPath = $derived(roverBodyPath(roverLengthPx, roverWidthPx))
  const poseDetail = $derived(`x ${rover.pose.x_m.toFixed(2)}m / y ${rover.pose.y_m.toFixed(2)}m / h ${radiansToDegrees(rover.pose.heading_rad).toFixed(0)}deg`)

  $effect(() => {
    if (!mapEl) return

    const resizeObserver = new ResizeObserver(([entry]) => {
      mapSize = {
        width: Math.max(320, entry.contentRect.width),
        height: Math.max(220, entry.contentRect.height - 32),
      }
    })

    resizeObserver.observe(mapEl)
    return () => resizeObserver.disconnect()
  })
</script>

<div class="map-widget" bind:this={mapEl}>
  <strong class="pose">{poseDetail}</strong>

  <svg viewBox={`0 0 ${mapSize.width} ${mapSize.height}`} preserveAspectRatio="none">
    <defs>
      <pattern id="moteus-grid" width="34" height="34" patternUnits="userSpaceOnUse">
        <path d="M 34 0 L 0 0 0 34" fill="none" stroke="rgba(255,255,255,.1)" stroke-width="1" />
      </pattern>
    </defs>
    <circle cx={mapSize.width / 2} cy={mapSize.height / 2} r={Math.min(mapSize.width, mapSize.height) * 0.48} fill="url(#moteus-grid)" />
    <line x1={mapSize.width / 2} y1="0" x2={mapSize.width / 2} y2={mapSize.height} class="origin" />
    <line x1="0" y1={mapSize.height / 2} x2={mapSize.width} y2={mapSize.height / 2} class="origin" />
    <polyline points={pathPoints} class="trail" />
    <g transform={robotTransform} class="robot">
      <path d={bodyPath} />
      <rect x={-roverLengthPx * 0.34} y={-roverWidthPx * 0.66} width={roverLengthPx * 0.22} height={roverWidthPx * 0.22} rx="3" />
      <rect x={-roverLengthPx * 0.34} y={roverWidthPx * 0.44} width={roverLengthPx * 0.22} height={roverWidthPx * 0.22} rx="3" />
      <rect x={roverLengthPx * 0.18} y={-roverWidthPx * 0.66} width={roverLengthPx * 0.22} height={roverWidthPx * 0.22} rx="3" />
      <rect x={roverLengthPx * 0.18} y={roverWidthPx * 0.44} width={roverLengthPx * 0.22} height={roverWidthPx * 0.22} rx="3" />
      <circle cx={roverLengthPx * 0.22} cy="0" r={Math.max(2, roverWidthPx * 0.08)} />
    </g>
  </svg>
</div>

<style>
  .map-widget {
    min-height: 100%;
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 0.6rem;
    overflow: hidden;
  }
  .pose {
    color: var(--accent);
    font-size: 0.75rem;
    text-align: right;
    text-transform: uppercase;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: 0.55rem;
    background: var(--bgDark);
  }
  .origin {
    stroke: color-mix(in srgb, var(--accent) 35%, transparent);
    stroke-dasharray: 7 9;
  }
  .trail {
    fill: none;
    stroke: var(--warning);
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 5;
  }
  .robot path {
    fill: var(--text);
  }
  .robot rect {
    fill: var(--bgDark);
    stroke: var(--accent);
    stroke-width: 1.5;
  }
  .robot circle {
    fill: var(--success);
  }
</style>
