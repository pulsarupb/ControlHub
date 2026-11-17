<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import DriveBar from "./DriveBar.svelte"
</script>

<div class="bar-widget">
  <strong class="readout">Steer {rover.steering.toFixed(2)}</strong>
  <div class="bar-slot"><DriveBar axis="steering" /></div>
  <div class="labels"><span>Spin</span></div>
</div>

<style>
  .bar-widget {
    min-height: 100%;
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 0.5rem;
    overflow: hidden;
  }
  .readout {
    color: var(--accent);
    font-size: 0.82rem;
    text-align: right;
    text-transform: uppercase;
  }
  .bar-slot {
    display: flex;
    min-height: 0;
    min-width: 0;
  }
  .labels {
    font-size: 0.7rem;
    color: var(--textDim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    text-align: center;
  }
</style>
