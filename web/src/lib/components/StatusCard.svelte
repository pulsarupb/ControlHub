<script lang="ts">
  import type { ConnectionState } from "$lib/types";

  let { connectionState, stopped }: { connectionState: ConnectionState; stopped: boolean } = $props();
</script>

<section class="hero">
  <div class="status-card">
    <div class="wanted" aria-label="Rover alert level">
      <span class:lit={stopped}>*</span>
      <span class:lit={connectionState === "offline"}>*</span>
      <span>*</span>
      <span>*</span>
      <span>*</span>
    </div>
    <span class={`status-dot ${connectionState}`}></span>
    <strong>{connectionState}</strong>
    <span>{stopped ? "motion locked" : "ready for motion"}</span>
  </div>
</section>

<style>
  .hero {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  .status-card {
    display: grid;
    gap: 0.35rem;
    min-width: 13rem;
    border: 2px solid rgba(255, 255, 255, 0.28);
    border-radius: 8px;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.04)), rgba(8, 18, 28, 0.78);
    box-shadow:
      0 0 0 3px rgba(0, 0, 0, 0.22),
      0 14px 36px rgba(0, 0, 0, 0.32);
    padding: 0.75rem;
    font-family: "Courier New", monospace;
    text-transform: uppercase;
    backdrop-filter: blur(10px) saturate(1.2);
  }

  .wanted {
    display: flex;
    gap: 0.25rem;
    color: rgba(255, 255, 255, 0.28);
    font-family: Georgia, "Times New Roman", serif;
    font-size: 1.4rem;
    line-height: 1;
  }

  .wanted .lit {
    color: #ffcf4f;
    text-shadow: 0 0 10px rgba(255, 207, 79, 0.9);
  }

  .status-dot {
    width: 0.85rem;
    height: 0.85rem;
    border-radius: 999px;
    background: #a9a9a9;
  }

  .status-dot.online {
    background: #39ff8f;
    box-shadow: 0 0 14px rgba(57, 255, 143, 0.86);
  }

  .status-dot.offline {
    background: #ff2136;
    box-shadow: 0 0 14px rgba(255, 33, 54, 0.86);
  }

  @media (max-width: 720px) {
    .hero {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
