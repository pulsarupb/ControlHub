<script lang="ts">
  import { ROVER_LENGTH_M, ROVER_WIDTH_M } from "$lib/constants";
  import { buildPathPoints, mapScale, robotPoseTransform, roverBodyPath } from "$lib/map";
  import { radiansToDegrees } from "$lib/math";
  import CardTitle from "$lib/components/CardTitle.svelte";
  import type { MapSize, PathPoint, Pose } from "$lib/types";

  let { pose, path }: { pose: Pose; path: PathPoint[] } = $props();
  let mapEl: HTMLElement | undefined = $state();
  let mapSize: MapSize = $state({ width: 640, height: 360 });

  const currentMapScale = $derived(mapScale(path.length ? path : [pose], mapSize));
  const pathPoints = $derived(buildPathPoints(path, mapSize));
  const robotTransform = $derived(robotPoseTransform(pose, mapSize, currentMapScale));
  const roverLengthPx = $derived(ROVER_LENGTH_M * currentMapScale);
  const roverWidthPx = $derived(ROVER_WIDTH_M * currentMapScale);
  const bodyPath = $derived(roverBodyPath(roverLengthPx, roverWidthPx));
  const poseDetail = $derived(`x ${pose.x_m.toFixed(2)}m / y ${pose.y_m.toFixed(2)}m / h ${radiansToDegrees(pose.heading_rad).toFixed(0)}deg`);

  $effect(() => {
    if (!mapEl) return;

    const resizeObserver = new ResizeObserver(([entry]) => {
      mapSize = {
        width: Math.max(320, entry.contentRect.width),
        height: Math.max(220, Math.min(420, entry.contentRect.height - 44)),
      };
    });

    resizeObserver.observe(mapEl);
    return () => resizeObserver.disconnect();
  });
</script>

<article class="map-card" bind:this={mapEl}>
  <CardTitle title="Minimap Prediction" detail={poseDetail} />

  <svg viewBox={`0 0 ${mapSize.width} ${mapSize.height}`} preserveAspectRatio="none">
    <defs>
      <pattern id="grid" width="34" height="34" patternUnits="userSpaceOnUse">
        <path d="M 34 0 L 0 0 0 34" fill="none" stroke="rgba(36,48,65,.12)" stroke-width="1" />
      </pattern>
    </defs>
    <circle cx={mapSize.width / 2} cy={mapSize.height / 2} r={Math.min(mapSize.width, mapSize.height) * 0.48} fill="url(#grid)" />
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
</article>

<style>
  .map-card {
    min-height: 0;
    height: 100%;
    border: 2px solid rgba(255, 255, 255, 0.28);
    border-radius: 8px;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.04)), rgba(8, 18, 28, 0.78);
    box-shadow:
      0 18px 48px rgba(0, 0, 0, 0.42),
      inset 0 0 0 1px rgba(0, 0, 0, 0.42);
    padding: 0.8rem;
    overflow: hidden;
    backdrop-filter: blur(10px) saturate(1.2);
  }

  svg {
    display: block;
    width: 100%;
    height: calc(100% - 2.35rem);
    min-height: 0;
    border: 2px solid rgba(255, 255, 255, 0.22);
    border-radius: 50%;
    background: radial-gradient(circle, rgba(28, 66, 74, 0.92), rgba(7, 17, 22, 0.94));
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

  @media (max-width: 1100px) {
    .map-card {
      grid-column: 1 / -1;
    }
  }

  @media (max-width: 720px) {
    .map-card {
      height: auto;
      min-height: 20rem;
    }
  }
</style>
