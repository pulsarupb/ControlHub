<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import DriveBar from "./DriveBar.svelte"
</script>

<div class="joystick-widget">
  <strong class="readout">T {rover.throttle.toFixed(2)} / S {rover.steering.toFixed(2)}</strong>

  <div class="sliders">
    <div class="bar-slot"><DriveBar axis="throttle" /></div>
    <div class="bar-slot"><DriveBar axis="steering" /></div>
  </div>

  <div class="labels">
    <span>Forward / Back</span>
    <span>Spin</span>
  </div>
</div>

<style>
  .joystick-widget {
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
  .sliders {
    display: flex;
    gap: 0.5rem;
    min-height: 0;
  }
  .bar-slot {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
  }
  .labels {
    display: flex;
    gap: 0.5rem;
    font-size: 0.7rem;
    color: var(--textDim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .labels span {
    flex: 1;
    text-align: center;
  }
</style>
