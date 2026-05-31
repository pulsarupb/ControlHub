<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
</script>

<span class="pill" class:offline={rover.connectionState === "offline"} class:stopped={rover.stopped}>
  {rover.connectionState} / {rover.stopped ? "locked" : "ready"}
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
