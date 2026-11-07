<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  const reason = $derived(rover.status?.devices.drive.stop_reason?.reason ?? "")
</script>

<span class="pill" class:offline={rover.connectionState === "offline"} class:stopped={rover.stopped} title={reason || undefined}>
  {rover.connectionState} / drive {rover.stopped ? "stopped" : "enabled"}
  {#if rover.status?.simulate} / SIMULATION{/if}
</span>

<style>
  .pill {
    border: 1px solid color-mix(in srgb, var(--success) 55%, transparent);
    border-radius: 0.4rem;
    padding: 0.35rem 0.75rem;
    background: color-mix(in srgb, var(--success) 10%, transparent);
    color: #cfe0c8;
    white-space: nowrap;
  }
  .pill.offline,
  .pill.stopped {
    border-color: color-mix(in srgb, var(--warning) 60%, transparent);
    background: color-mix(in srgb, var(--warning) 12%, transparent);
    color: #e5cf9e;
  }
</style>
