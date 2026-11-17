<script lang="ts">
  import { onMount } from "svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"
  import { apiFetch } from "$lib/api-client"
  import DeviceStatus from "./DeviceStatus.svelte"

  let kind = $state("color")
  let url = $state("")
  let failed = $state(false)

  const zed = $derived(rover.status?.devices.zed)
  // One status line for every state, so the frame area never shows two messages at once.
  const message = $derived(
    !zed?.connected ? "Camera offline"
    : !zed.data.camera_ready ? "Camera starting"
    : failed ? "No frames"
    : "Waiting for frames",
  )

  onMount(() => {
    let alive = true
    let busy = false
    const tick = async () => {
      if (busy || !zed?.connected || !zed.data.camera_ready) return
      busy = true
      try {
        const response = await apiFetch(roverConnection.apiUrl(`/api/zed/frame/${kind}`))
        if (!response.ok) throw new Error(`HTTP ${response.status}`)
        const next = URL.createObjectURL(await response.blob())
        if (!alive) {
          URL.revokeObjectURL(next)
          return
        }
        if (url) URL.revokeObjectURL(url)
        url = next
        failed = false
      } catch {
        failed = true
      } finally {
        busy = false
      }
    }
    const timer = setInterval(tick, 150)
    return () => {
      alive = false
      clearInterval(timer)
      if (url) URL.revokeObjectURL(url)
    }
  })
</script>

<div class="zed">
  <header class="widget-header">
    <strong>ZED video</strong>
    <select aria-label="Video source" bind:value={kind}>
      <option value="color">Color</option>
      <option value="depth">Depth</option>
    </select>
  </header>
  <DeviceStatus device={rover.status?.devices.zed} />
  <div class="frame">
    {#if url && zed?.connected}
      <img src={url} alt={`ZED ${kind}`} />
    {:else}
      <span>{message}</span>
    {/if}
  </div>
</div>

<style>
  .zed {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .frame {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    margin-top: 0.6rem;
    border: 1px solid var(--border);
    border-radius: 0.4rem;
    background: var(--bgDark);
    overflow: hidden;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .frame span {
    font-size: 0.85rem;
    color: var(--textSubtle);
  }
</style>
