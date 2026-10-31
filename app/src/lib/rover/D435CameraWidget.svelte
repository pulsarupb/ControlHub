<script lang="ts">
  import { tick as flush } from "svelte"
  import { apiFetch } from "$lib/api-client"
  import { roverConnection } from "$lib/rover-connection.svelte"
  import { rover } from "$lib/rover-control.svelte"

  let crosshair = $state(false)
  let url = $state("")
  let error = $state("")
  // Status objects are replaced on every telemetry poll. Only restart capture
  // when simulation actually changes, not whenever a new status object arrives.
  const simulated = $derived(rover.status?.simulate ?? false)

  $effect(() => {
    const kind = crosshair ? "crosshair" : "color"
    const baseUrl = roverConnection.baseUrl
    const online = roverConnection.state === "online"
    const simulation = simulated
    let alive = true
    let busy = false
    let pending: AbortController | undefined
    let currentUrl = ""
    let lastFrameAt = 0
    url = ""; error = ""
    if (!online || !baseUrl || simulation) return

    const tick = async () => {
      if (busy) return
      busy = true
      const controller = new AbortController()
      pending = controller
      const timeout = setTimeout(() => controller.abort(), 4000)
      let next = ""
      try {
        const response = await apiFetch(`${baseUrl}/api/d435/frame/${kind}`, { signal: controller.signal })
        if (!response.ok) {
          const detail = await response.json().catch(() => null)
          throw new Error(detail?.error || "D435 frame unavailable")
        }
        next = URL.createObjectURL(await response.blob())
        // Keep the visible frame intact until its replacement can be painted.
        const image = new Image()
        image.src = next
        await image.decode()
        if (!alive) return
        const previous = currentUrl
        currentUrl = next; url = next; error = ""
        next = ""
        lastFrameAt = Date.now()
        await flush()
        if (previous) URL.revokeObjectURL(previous)
      } catch (e) {
        if (alive) {
          // A single failed poll should not flash the offline placeholder.
          if (Date.now() - lastFrameAt >= 3000) {
            const previous = currentUrl
            currentUrl = ""; url = ""
            await flush()
            if (previous) URL.revokeObjectURL(previous)
          }
          error = e instanceof Error ? e.message : String(e)
        }
      } finally {
        if (next) URL.revokeObjectURL(next)
        clearTimeout(timeout); busy = false
      }
    }
    void tick()
    const timer = setInterval(tick, 150)
    return () => {
      alive = false; clearInterval(timer); pending?.abort()
      if (currentUrl) URL.revokeObjectURL(currentUrl)
    }
  })
</script>

<section>
  <header><strong>D435 Gripper Camera</strong><label><input type="checkbox" bind:checked={crosshair} /> Crosshair</label></header>
  <div class="image">
    {#if url}<img src={url} alt="RealSense D435 gripper camera" />
    {:else}<p>{roverConnection.state !== "online" ? "Rover offline" : simulated ? "SIMULATED · no D435 frames" : error || "Waiting for D435 camera"}</p>{/if}
  </div>
  <footer class:live={Boolean(url)} role="status">{url ? "Live · D435 color" : "Offline · connect D435 to rover"}</footer>
</section>

<style>
  section { padding: .7rem; height: 100%; min-height: 0; display: flex; flex-direction: column; gap: .5rem; }
  header { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: .5rem; }
  label { display: flex; align-items: center; gap: .3rem; font-size: .8rem; }
  .image { flex: 1; min-height: 0; overflow: hidden; display: flex; align-items: center; justify-content: center; background: #0005; border-radius: .25rem; }
  img { width: 100%; height: 100%; object-fit: contain; min-height: 0; }
  p { padding: .6rem; text-align: center; font-size: .85rem; opacity: .8; }
  footer { font-size: .75rem; opacity: .8; }
  .live { color: #9d9; }
</style>
