<script lang="ts">
  import { rover } from "$lib/data/rover.svelte"

  const points = $derived(
    rover.samples
      .map((sample, index) => {
        const x = rover.samples.length <= 1 ? 0 : (index / (rover.samples.length - 1)) * 100
        const y = 100 - Math.max(0, Math.min(100, sample.signal))
        return `${x},${y}`
      })
      .join(" ")
  )
</script>

<div class="graph">
  <header>
    <span>Signal Trend</span>
    <strong>{rover.signal.toFixed(0)}%</strong>
  </header>
  <svg viewBox="0 0 100 100" preserveAspectRatio="none">
    <polyline points={points} fill="none" stroke="var(--accent)" stroke-width="3" vector-effect="non-scaling-stroke" />
  </svg>
</div>

<style>
  .graph {
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 0.75rem;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  strong {
    font-size: 1.8rem;
  }
  svg {
    width: 100%;
    height: 100%;
    min-height: 120px;
    border: 1px solid var(--border);
    border-radius: 0.55rem;
    background: repeating-linear-gradient(to right, transparent 0 19%, rgba(255,255,255,0.07) 20%),
      repeating-linear-gradient(to top, transparent 0 24%, rgba(255,255,255,0.05) 25%),
      var(--bgDark);
  }
</style>
