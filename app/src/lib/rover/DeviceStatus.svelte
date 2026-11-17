<script lang="ts">
  import type { Device } from "$lib/hardware-types"
  import { roverConnection } from "$lib/rover-connection.svelte"
  let { device }: { device: Device<unknown> | undefined } = $props()
  // When the rover itself is unreachable, say so: a device under an offline rover is not "Disabled".
  const roverState = $derived(roverConnection.state)
  const label = $derived(
    roverState === "offline" ? "Rover offline"
    : roverState === "connecting" ? "Connecting…"
    : !device?.enabled ? "Disabled"
    : device.connected ? "Connected" : "Offline",
  )
</script>
<span class:status-error={Boolean(device?.error)}>
  {device?.simulated && roverState === "online" ? "SIMULATED · " : ""}
  {label}
  {#if device?.error} · {device.error}{/if}
</span>
<style>span { font-size: .8rem; opacity: .8; }</style>
