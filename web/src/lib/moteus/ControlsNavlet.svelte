<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
</script>

<span class="controls">
  T {rover.throttle.toFixed(2)} / S {rover.steering.toFixed(2)}
</span>

<style>
  .controls {
    border: 1px solid var(--bgLight);
    border-radius: 99rem;
    padding: 0.35rem 0.75rem;
    background: var(--bgDark);
    color: var(--accent);
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
    white-space: nowrap;
  }
</style>
