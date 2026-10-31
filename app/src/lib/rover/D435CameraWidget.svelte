<script lang="ts">
  import { apiFetch } from "$lib/api-client"
  import { roverConnection } from "$lib/rover-connection.svelte"
  import { rover } from "$lib/rover-control.svelte"

  let crosshair = $state(false)
  let url = $state("")
  let error = $state("")

  $effect(() => {
    const kind = crosshair ? "crosshair" : "color"
    const baseUrl = roverConnection.baseUrl
    const online = roverConnection.state === "online"
    const simulated = rover.status?.simulate
    let alive = true
    let busy = false
    let pending: AbortController | undefined
    let currentUrl = ""
    url = ""; error = ""
    if (!online || !baseUrl || simulated) return

    const tick = async () => {
      if (busy) return
      busy = true
      const controller = new AbortController()
      pending = controller
      const timeout = setTimeout(() => controller.abort(), 4000)
      try {
        const response = await apiFetch(`${baseUrl}/api/d435/frame/${kind}`, { signal: controller.signal })
        if (!response.ok) {
          const detail = await response.json().catch(() => null)
          throw new Error(detail?.error || "D435 frame unavailable")
        }
        const next = URL.createObjectURL(await response.blob())
        if (!alive) { URL.revokeObjectURL(next); return }
        if (currentUrl) URL.revokeObjectURL(currentUrl)
        currentUrl = next; url = next; error = ""
      } catch (e) {
        if (alive) {
          if (currentUrl) URL.revokeObjectURL(currentUrl)
          currentUrl = ""; url = ""; error = e instanceof Error ? e.message : String(e)
        }
      } finally { clearTimeout(timeout); busy = false }
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
    {:else}<p>{roverConnection.state !== "online" ? "Rover offline" : rover.status?.simulate ? "SIMULATED · no D435 frames" : error || "Waiting for D435 camera"}</p>{/if}
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
