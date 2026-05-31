<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
</script>

<div class="telemetry-card">
  {#if rover.status?.last_error}
    <strong class="status">{rover.status.last_error}</strong>
  {/if}
  <div class="motor-grid">
    {#each rover.status?.motors ?? [] as motor}
      <div class="motor">
        <strong>Motor {motor.id}</strong>
        <span>pos {motor.position.toFixed(3)}</span>
        <span>vel {motor.velocity.toFixed(3)}</span>
        <span>fault {motor.fault}</span>
      </div>
    {:else}
      <p>Waiting for motor telemetry...</p>
    {/each}
  </div>
</div>

<style>
  .telemetry-card {
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 0.7rem;
    overflow: auto;
  }
  .status {
    color: var(--accent);
    font-size: 0.82rem;
    text-align: right;
    text-transform: uppercase;
  }
  .motor-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
    align-content: start;
  }
  .motor {
    display: grid;
    gap: 0.35rem;
    border: 1px solid var(--border);
    border-radius: 0.45rem;
    background: var(--bgDark);
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
    padding: 0.65rem;
  }
  .motor span,
  p {
    color: var(--textMuted);
    font-size: 0.88rem;
  }
  p {
    margin: 0;
  }
</style>
