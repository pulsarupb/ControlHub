<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
</script>

<div class="emergency-widget">
  <button class="estop" onclick={rover.stopRover} aria-label="Emergency stop all rover motors">
    <strong>STOP</strong>
  </button>
  <button class="reset" onclick={rover.resetRover}>Reset Rover</button>
</div>

<style>
  .emergency-widget {
    min-height: 100%;
    display: grid;
    gap: 0.65rem;
    align-content: start;
  }
  .estop {
    min-height: clamp(9rem, 22vh, 14rem);
    border: 1px solid #a6433d;
    border-radius: 0.55rem;
    background: #9f302d;
    color: white;
    cursor: pointer;
    font-family: inherit;
    letter-spacing: 0.04em;
    box-shadow: inset 0 -3px 0 rgba(0, 0, 0, 0.24);
  }
  .estop:hover {
    background: #b43a35;
  }
  .estop strong {
    display: block;
  }
  .estop strong {
    font-size: clamp(2.8rem, 7vw, 5.5rem);
    line-height: 0.95;
  }
  .reset {
    border: 1px solid var(--borderStrong);
    border-radius: 0.45rem;
    background: var(--surfaceRaised);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-weight: 700;
    padding: 0.8rem;
    text-transform: uppercase;
  }
</style>
