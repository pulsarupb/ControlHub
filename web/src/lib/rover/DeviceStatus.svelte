<script lang="ts">
  import type { Device } from "$lib/hardware-types"
  let { device }: { device: Device<unknown> | undefined } = $props()
</script>
<span class:error={Boolean(device?.error)}>
  {device?.simulated ? "SIMULATED · " : ""}
  {!device?.enabled ? "Disabled" : device.connected ? "Connected" : "Offline"}
  {#if device?.error} · {device.error}{/if}
</span>
<style>span { font-size: .8rem; opacity: .8; }.error { color: #f99; }</style>
