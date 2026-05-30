<script lang="ts">
  import CardTitle from "$lib/components/CardTitle.svelte";
  import type { RoverStatus } from "$lib/types";

  let { status }: { status: RoverStatus | null } = $props();
</script>

<article class="telemetry-card">
  <CardTitle title="Garage Telemetry" detail={status?.last_error ?? "nominal"} />
  <div class="motor-grid">
    {#each status?.motors ?? [] as motor}
      <div class="motor">
        <strong>Motor {motor.id}</strong>
        <span>pos {motor.position.toFixed(3)}</span>
        <span>vel {motor.velocity.toFixed(3)}</span>
        <span>fault {motor.fault}</span>
      </div>
    {/each}
  </div>
</article>

<style>
  .telemetry-card {
    grid-column: 1 / -1;
    max-height: 9.5rem;
    min-height: 0;
    border: 2px solid rgba(255, 255, 255, 0.28);
    border-radius: 8px;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.04)), rgba(8, 18, 28, 0.78);
    box-shadow:
      0 18px 48px rgba(0, 0, 0, 0.42),
      inset 0 0 0 1px rgba(0, 0, 0, 0.42);
    padding: 0.8rem;
    overflow: hidden;
    backdrop-filter: blur(10px) saturate(1.2);
  }

  .motor-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .motor {
    display: grid;
    gap: 0.35rem;
    border: 1px solid rgba(143, 232, 255, 0.24);
    border-radius: 8px;
    background: rgba(4, 12, 20, 0.48);
    font-family: "Courier New", monospace;
    padding: 0.65rem;
  }

  .motor span {
    color: #b9d3d0;
    font-size: 0.88rem;
  }

  @media (max-width: 720px) {
    .telemetry-card {
      max-height: none;
    }

    .motor-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
