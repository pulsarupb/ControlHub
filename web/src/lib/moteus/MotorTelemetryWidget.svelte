<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
</script>

<div class="telemetry-card">
  <header>
    <span>Motor Telemetry</span>
    <strong>{rover.status?.last_error ?? "nominal"}</strong>
  </header>
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
  header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
    text-transform: uppercase;
  }
  header strong {
    color: var(--accent);
    font-size: 0.82rem;
    text-align: right;
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
    border: 1px solid rgba(56, 189, 248, 0.24);
    border-radius: 0.75rem;
    background: rgba(4, 12, 20, 0.48);
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
    padding: 0.65rem;
  }
  .motor span,
  p {
    color: rgba(255, 255, 255, 0.72);
    font-size: 0.88rem;
  }
  p {
    margin: 0;
  }
</style>
