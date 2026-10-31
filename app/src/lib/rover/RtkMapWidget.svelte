<script lang="ts">
  import { onMount, untrack } from "svelte"
  import { base } from "$app/paths"
  import { rover } from "$lib/rover-control.svelte"
  import DeviceStatus from "./DeviceStatus.svelte"
  import { CAMPUS, insideMap, localPoint, validCoordinates, type OfflineMap, type Point } from "./rtk-map"

  const device = $derived(rover.status?.devices.rtk)
  const fix = $derived(device?.data.fix)
  const names = ["No Fix", "GPS", "DGPS", "PPS", "RTK Fixed", "RTK Float", "Dead reckoning", "Manual", "Simulation"]
  let map = $state.raw<OfflineMap | null>(null)
  let features = $state.raw<{ feature: OfflineMap["features"][number]; bounds: number[] }[]>([])
  let error = $state("")
  let now = $state(Date.now() / 1000)
  let clock = $state({ server: 0, client: 0 })
  let host: HTMLDivElement
  let canvas: HTMLCanvasElement
  let width = $state(600), height = $state(400)
  let zoom = $state(.32)
  let center = $state<Point>([0, 0])
  let follow = $state(true)
  let trail = $state<{ point: Point; time: number }[]>([])
  let drag: { id: number; x: number; y: number; center: Point } | null = null
  const position = $derived(fix && fix.quality > 0 && validCoordinates(fix.latitude, fix.longitude)
    ? localPoint(fix.latitude!, fix.longitude!) : null)
  const age = $derived(fix ? Math.max(0, clock.server + now - clock.client - fix.received_at) : null)
  const stale = $derived(rover.connectionState !== "online" || !device?.connected || age === null || age > 3)
  const color = $derived(stale || !position ? "#ff8b8b" : fix?.quality === 4 ? "#4de0ab" : fix?.quality === 5 ? "#f7ce69" : "#70caff")
  const outside = $derived(Boolean(map && position && fix && !insideMap(fix.latitude!, fix.longitude!, map.bounds)))
  const marker = $derived(position ? screen(position) : null)
  const trailPath = $derived(trail.map((p, i) => `${i ? "L" : "M"}${screen(p.point).join(",")}`).join(" "))
  // Mercator scale is larger than ground scale at this latitude.
  const scaleMeters = $derived(100 / zoom * Math.cos(CAMPUS[0] * Math.PI / 180))

  function screen(p: Point): Point { return [(p[0] - center[0]) * zoom + width / 2, (p[1] - center[1]) * zoom + height / 2] }
  function changeZoom(factor: number) { zoom = Math.max(.025, Math.min(12, zoom * factor)) }
  function recenter() { follow = true; if (position) center = [...position] }
  function campus() { follow = false; center = [0, 0]; zoom = .32 }
  function pointerDown(event: PointerEvent) {
    if (event.button !== 0) return
    host.setPointerCapture(event.pointerId)
    follow = false
    drag = { id: event.pointerId, x: event.clientX, y: event.clientY, center: [...center] }
  }
  function pointerMove(event: PointerEvent) {
    if (!drag || drag.id !== event.pointerId) return
    center = [drag.center[0] - (event.clientX - drag.x) / zoom, drag.center[1] - (event.clientY - drag.y) / zoom]
  }
  function keyDown(event: KeyboardEvent) {
    const offsets: Record<string, Point> = { ArrowLeft: [-80, 0], ArrowRight: [80, 0], ArrowUp: [0, -80], ArrowDown: [0, 80] }
    const offset = offsets[event.key]
    if (!offset) return
    event.preventDefault(); follow = false
    center = [center[0] + offset[0] / zoom, center[1] + offset[1] / zoom]
  }

  $effect(() => {
    if (follow && position && untrack(() => center[0] !== position[0] || center[1] !== position[1])) center = [...position]
  })
  $effect(() => {
    if (rover.status) clock = { server: rover.status.time, client: Date.now() / 1000 }
  })
  $effect(() => {
    if (!position || stale || !fix) return
    const time = fix.received_at
    // Only append new receiver fixes; never bridge a telemetry outage with a line.
    const history = untrack(() => trail)
    const previous = history.at(-1)
    if (previous?.time === time) return
    trail = [...(previous && (time < previous.time || time - previous.time > 3) ? [] : history), { point: position as Point, time }].slice(-2000)
  })

  onMount(() => {
    const controller = new AbortController()
    void fetch(`${base}/maps/politehnica.json`, { signal: controller.signal })
      .then(response => { if (!response.ok) throw new Error(`Offline map: HTTP ${response.status}`); return response.json() })
      .then((data: OfflineMap) => {
        if (controller.signal.aborted) return
        const order = ["green", "water", "building", "waterway", "rail", "road"]
        features = data.features.map(feature => {
          const bounds = [Infinity, Infinity, -Infinity, -Infinity]
          for (const [x, y] of feature.points) {
            bounds[0] = Math.min(bounds[0], x); bounds[1] = Math.min(bounds[1], y)
            bounds[2] = Math.max(bounds[2], x); bounds[3] = Math.max(bounds[3], y)
          }
          return { feature, bounds }
        }).sort((a, b) => order.indexOf(a.feature.kind) - order.indexOf(b.feature.kind))
        map = data
      })
      .catch(e => { if (!controller.signal.aborted) error = String(e) })
    const observer = new ResizeObserver(() => { width = host.clientWidth; height = host.clientHeight })
    observer.observe(host)
    const timer = setInterval(() => { now = Date.now() / 1000 }, 500)
    return () => { controller.abort(); observer.disconnect(); clearInterval(timer) }
  })

  $effect(() => {
    if (!canvas || !map) return
    const ratio = Math.min(globalThis.devicePixelRatio || 1, 2)
    canvas.width = Math.max(1, width * ratio); canvas.height = Math.max(1, height * ratio)
    const ctx = canvas.getContext("2d")
    if (!ctx) return
    ctx.scale(ratio, ratio)
    ctx.fillStyle = "#1b2530"; ctx.fillRect(0, 0, width, height)
    const labels: { text: string; x: number; y: number }[] = []
    const left = center[0] - (width / 2 + 100) / zoom, right = center[0] + (width / 2 + 100) / zoom
    const top = center[1] - (height / 2 + 100) / zoom, bottom = center[1] + (height / 2 + 100) / zoom
    ctx.translate(width / 2, height / 2); ctx.scale(zoom, zoom); ctx.translate(-center[0], -center[1])
    for (const { feature: f, bounds: b } of features) {
      const kind = f.kind
      if ((kind === "building" && zoom < .12) || b[2] < left || b[0] > right || b[3] < top || b[1] > bottom) continue
      ctx.beginPath(); f.points.forEach((p, i) => i ? ctx.lineTo(...p) : ctx.moveTo(...p))
      if (["green", "water", "building"].includes(kind)) {
        ctx.closePath(); ctx.fillStyle = kind === "green" ? "#244b40" : kind === "water" ? "#204d6b" : "#374554"; ctx.fill()
        if (kind === "building") { ctx.strokeStyle = "#465668"; ctx.lineWidth = .6 / zoom; ctx.stroke() }
      } else {
        const major = /^(motorway|trunk|primary|secondary|tertiary)(_link)?$/.test(f.detail)
        const foot = /footway|path|steps|cycleway|pedestrian/.test(f.detail)
        ctx.lineCap = "round"; ctx.lineJoin = "round"
        ctx.strokeStyle = kind === "waterway" ? "#3678a1" : kind === "rail" ? "#8c839b" : foot ? "#718780" : major ? "#c5ab7d" : "#768592"
        ctx.lineWidth = (kind === "road" ? Math.max(foot ? .7 : 1.2, Math.min(major ? 14 : 9, zoom * (major ? 14 : 7))) : Math.max(1, zoom * 3)) / zoom
        ctx.stroke()
        if (kind === "road" && major && f.name && zoom > .12) {
          const p = screen(f.points[Math.floor(f.points.length / 2)])
          if (p[0] > 30 && p[0] < width - 30 && p[1] > 20 && p[1] < height - 20) labels.push({ text: f.name, x: p[0], y: p[1] })
        }
      }
    }
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0)
    ctx.font = "11px system-ui"; ctx.textAlign = "center"; ctx.lineWidth = 3
    const placed: { x: number; y: number; half: number }[] = []
    const named = new Set<string>()
    for (const label of labels) {
      const half = ctx.measureText(label.text).width / 2
      if (named.has(label.text) || placed.some(p => Math.abs(p.x - label.x) < p.half + half + 12 && Math.abs(p.y - label.y) < 24)) continue
      ctx.strokeStyle = "#17212d"; ctx.strokeText(label.text, label.x, label.y)
      ctx.fillStyle = "#e4e7e9"; ctx.fillText(label.text, label.x, label.y)
      placed.push({ ...label, half })
      named.add(label.text)
    }
  })
</script>

<section>
  <header><strong>RTK Map <small>· Politehnica, Bucharest</small></strong><DeviceStatus {device} /></header>
  <div class="toolbar">
    <button class:active={follow} onclick={() => { follow = !follow }}>Follow rover</button>
    <button onclick={recenter} disabled={!position}>Recenter</button>
    <button onclick={campus}>Campus</button>
    <button onclick={() => changeZoom(1.5)} aria-label="Zoom in">+</button>
    <button onclick={() => changeZoom(1 / 1.5)} aria-label="Zoom out">−</button>
    <button onclick={() => trail = []}>Clear trail</button>
  </div>
  <!-- Custom map surface supports keyboard panning; application is intentional, not a button. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div class="viewport" bind:this={host} role="application" tabindex="0" onkeydown={keyDown} aria-label="Offline RTK map. Drag or use arrow keys to pan; use zoom buttons or mouse wheel."
    onpointerdown={pointerDown} onpointermove={pointerMove} onpointerup={() => drag = null} onpointercancel={() => drag = null} onlostpointercapture={() => drag = null}
    onwheel={(event) => { event.preventDefault(); changeZoom(Math.exp(-event.deltaY * .002)) }}>
    <canvas bind:this={canvas}></canvas>
    <svg width={width} height={height} aria-label="Rover position and movement trail">
      <path d={trailPath} fill="none" stroke="#6fe3cc" stroke-width="2.5" opacity=".8" />
      {#if marker}
        <circle cx={marker[0]} cy={marker[1]} r="17" fill={color} opacity=".15" />
        <circle cx={marker[0]} cy={marker[1]} r="7" fill={color} stroke="#fff" stroke-width="2" />
        <text x={marker[0] + 13} y={marker[1] - 13} fill={color} stroke="#17212d" stroke-width="3" paint-order="stroke" font-size="12">{stale ? "Last fix" : "Rover"}</text>
      {/if}
    </svg>
    <span class="north">N ↑</span>
    <span class="scale"><span></span>{scaleMeters < 10 ? scaleMeters.toFixed(1) : Math.round(scaleMeters)} m</span>
    {#if error}<p class="message warning" role="alert">Map unavailable: {error}</p>
    {:else if !map}<p class="message">Loading bundled offline map…</p>
    {:else if outside}<p class="message warning">Rover is outside the bundled map. Use Campus to return.</p>
    {:else if !position}<p class="message">Waiting for a valid RTK position · map available offline</p>{/if}
    <a class="attribution" href="https://www.openstreetmap.org/copyright" target="_blank" rel="noreferrer">© OpenStreetMap contributors · ODbL</a>
  </div>
  <footer>
    <span style:color>{device?.simulated ? "SIMULATED · " : ""}{names[fix?.quality ?? 0] ?? "Unknown fix"}{stale ? " · STALE / disconnected" : ""}</span>
    <span>{age === null ? "No fix" : `${age.toFixed(1)} s old`} · {fix?.satellites ?? "—"} satellites</span>
    <span>{position ? `${fix!.latitude!.toFixed(7)}°, ${fix!.longitude!.toFixed(7)}°` : "No valid coordinates"}</span>
  </footer>
</section>

<style>
  section { padding: .6rem; height: 100%; min-height: 0; box-sizing: border-box; display: flex; flex-direction: column; gap: .4rem; }
  header, .toolbar, footer { display: flex; align-items: center; flex-wrap: wrap; gap: .4rem; }
  header { justify-content: space-between; } small { font-size: .7rem; font-weight: normal; opacity: .7; }
  button { background: #202b38; color: inherit; border: 1px solid #465668; border-radius: .35rem; padding: .25rem .5rem; cursor: pointer; font-size: .75rem; }
  button.active { border-color: #4de0ab; color: #4de0ab; } button:disabled { opacity: .4; cursor: default; }
  .viewport { flex: 1; min-height: 140px; position: relative; overflow: hidden; background: #1b2530; border-radius: .5rem; cursor: grab; touch-action: none; }
  .viewport:active { cursor: grabbing; } canvas, svg { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
  .north, .scale, .attribution { position: absolute; background: #17212de6; border-radius: .2rem; padding: .15rem .3rem; color: #dce7ee; font-size: .65rem; }
  .north { top: .5rem; right: .5rem; } .scale { left: .5rem; bottom: 1.8rem; text-align: center; }
  .scale span { display: block; width: 100px; height: 5px; border: 1px solid #dce7ee; border-top: 0; margin-bottom: .15rem; }
  .attribution { right: 0; bottom: 0; cursor: pointer; } .message { position: absolute; left: .5rem; top: .5rem; right: 3rem; width: fit-content; margin: 0; padding: .4rem .6rem; background: #17212de6; color: #dce7ee; font-size: .75rem; border-radius: .3rem; pointer-events: none; }
  .warning { color: #ffd58c; } footer { justify-content: space-between; font-size: .7rem; font-variant-numeric: tabular-nums; }
</style>
