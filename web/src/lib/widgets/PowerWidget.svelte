<script lang="ts">
  import { rover } from "$lib/data/rover.svelte"

  const batteryLevel = $derived(Math.max(0, Math.min(100, rover.battery)))
</script>

<div class="power">
  <div>
    <span>Battery</span>
    <strong>{batteryLevel.toFixed(0)}%</strong>
  </div>
  <div class="bar"><span style="width: {batteryLevel}%"></span></div>
  <p>{rover.voltage.toFixed(2)} V bus voltage</p>
</div>

<style>
  .power {
    height: 100%;
    display: grid;
    align-content: center;
    gap: 1rem;
  }
  div:first-child {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  span {
    color: var(--textMuted);
  }
  strong {
    font-size: clamp(2.4rem, 8vw, 5rem);
    line-height: 1;
  }
  .bar {
    height: 1rem;
    overflow: hidden;
    border-radius: 0.35rem;
    background: var(--bgDark);
    border: 1px solid var(--border);
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--success);
  }
  p {
    margin: 0;
    color: var(--textMuted);
  }
</style>
