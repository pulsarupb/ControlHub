<script lang="ts">
  import { apiFetch } from "$lib/api-client"
  import { roverConnection } from "$lib/rover-connection.svelte"
  import type { OakCamera } from "$lib/hardware-types"

  let { slot, name, camera, enabled, simulated }: {
    slot: number; name: string; camera: OakCamera | undefined; enabled: boolean; simulated: boolean;
  } = $props()
  let kind = $state("color")
  let url = $state("")
  let error = $state("")
  const connected = $derived(enabled && !simulated && Boolean(camera?.connected && camera.updated_at
    && Date.now() / 1000 - camera.updated_at < 3))
  const mxid = $derived(camera?.mxid ?? "")
  const distance = (mm: number | null | undefined) => mm == null || mm <= 0 ? "—" : `${(mm / 1000).toFixed(2)} m`

  $effect(() => {
    const source = kind
    const baseUrl = roverConnection.baseUrl
    const identity = mxid
    const online = connected && roverConnection.state === "online"
    const index = slot
    let alive = true, busy = false
    let pending: AbortController | undefined
    let currentUrl = ""
    url = ""; error = ""
    if (!online || !baseUrl || !identity) return
    const tick = async () => {
      if (busy) return
      busy = true
      const controller = new AbortController()
      pending = controller
      const timeout = setTimeout(() => controller.abort(), 4000)
      try {
        const response = await apiFetch(`${baseUrl}/api/oak/frame/${index}/${source}`, { signal: controller.signal })
        if (!response.ok) throw new Error("No fresh frame available")
        const next = URL.createObjectURL(await response.blob())
        if (!alive) { URL.revokeObjectURL(next); return }
        if (currentUrl) URL.revokeObjectURL(currentUrl)
        currentUrl = next; url = next; error = ""
      } catch (e) {
        if (alive) {
          if (currentUrl) URL.revokeObjectURL(currentUrl)
          currentUrl = ""; url = ""; error = String(e)
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

<article>
  <header><strong>{name}</strong>
    <select aria-label={`${name} video source`} bind:value={kind}>
      <option value="color">Color</option><option value="depth">Depth / disparity</option>
    </select>
  </header>
  <div class="identity" title={mxid}>{mxid || "No device assigned"}</div>
  <div class="image">
    {#if url && connected}<img src={url} alt={`${name} ${kind} view`} />
    {:else}<p>{!enabled ? "Disabled · enable OAK-D in Settings" : simulated ? "SIMULATED · no camera frames" : camera?.error || "Waiting for camera"}</p>{/if}
  </div>
  <footer><span class:online={connected}>{connected ? "Live" : "Offline"}</span>
    <span>Min {distance(camera?.min_mm)} · Avg {distance(camera?.avg_mm)} · Max {distance(camera?.max_mm)}</span>
  </footer>
  {#if error}<p class="error" role="status">{error}</p>{/if}
</article>

<style>
  article { display: flex; flex-direction: column; min-width: 0; min-height: 180px; gap: .4rem; padding: .6rem; border: 1px solid #7774; border-radius: .4rem; background: #0002; }
  header, footer { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: .4rem; }
  strong { font-size: .9rem; }
  select { max-width: 100%; color: inherit; background: #222; border: 1px solid #666; border-radius: .25rem; padding: .25rem; }
  .identity { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: monospace; font-size: .7rem; opacity: .65; }
  .image { flex: 1; min-height: 100px; display: flex; align-items: center; justify-content: center; background: #0005; border-radius: .25rem; overflow: hidden; }
  img { display: block; width: 100%; height: 100%; max-height: 100%; object-fit: contain; min-height: 0; }
  p { margin: .4rem; font-size: .8rem; text-align: center; opacity: .8; }
  footer { font-size: .7rem; opacity: .8; }
  .online { color: #9d9; }
  .error { color: #f99; }
</style>
