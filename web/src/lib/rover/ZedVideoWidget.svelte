<script lang="ts">
  import { onMount } from "svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"
  import { apiFetch } from "$lib/api-client"
  import DeviceStatus from "./DeviceStatus.svelte"
  let kind = $state("color")
  let url = $state("")
  let error = $state("")
  onMount(() => {
    let alive = true, busy = false
    const tick = async () => {
      if (busy || !rover.status?.devices.zed.connected || !rover.status.devices.zed.data.camera_ready) return
      busy = true
      try {
        const response = await apiFetch(roverConnection.apiUrl(`/api/zed/frame/${kind}`))
        if (!response.ok) throw new Error("No camera frame available")
        const next = URL.createObjectURL(await response.blob())
        if (!alive) { URL.revokeObjectURL(next); return }
        if (url) URL.revokeObjectURL(url)
        url = next; error = ""
      } catch (e) { error = String(e) }
      finally { busy = false }
    }
    const timer = setInterval(tick, 150)
    return () => { alive = false; clearInterval(timer); if (url) URL.revokeObjectURL(url) }
  })
</script>
<section><header><strong>ZED video</strong><select aria-label="Video source" bind:value={kind}><option value="color">Color</option><option value="depth">Depth</option></select></header>
  <DeviceStatus device={rover.status?.devices.zed} />
  {#if url && rover.status?.devices.zed.connected}<img src={url} alt={`ZED ${kind}`} />{:else}<p>No live frame</p>{/if}
  {#if error}<p>{error}</p>{/if}
</section>
<style>section { padding: .7rem; height: 100%; display: flex; flex-direction: column; gap: .5rem; }header { display: flex; justify-content: space-between; }img { width: 100%; min-height: 0; flex: 1; object-fit: contain; }select { color: inherit; background: #222; }</style>
