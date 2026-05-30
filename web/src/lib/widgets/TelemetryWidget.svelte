<script lang="ts">
  import { rover } from "$lib/data/rover.svelte"
</script>

<div class="telemetry">
  {#each rover.telemetry as line}
    <p>{line}</p>
  {/each}
</div>

<style>
  .telemetry {
    display: grid;
    gap: 0.45rem;
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
  }
  p {
    margin: 0;
    padding: 0.45rem 0.6rem;
    border-radius: 0.55rem;
    background: rgba(255, 255, 255, 0.04);
  }
</style>
