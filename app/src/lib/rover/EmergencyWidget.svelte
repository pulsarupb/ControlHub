<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
</script>

<div class="emergency-widget">
  <button class="estop" onclick={() => rover.stopRover()} aria-label="Stop drive and arm">
    <strong>STOP</strong>
  </button>
  <button class="reset" onclick={rover.resetRover}>Enable Drive</button>
  {#if rover.status?.devices.drive.stop_reason}
    <p class="reason">Stopped: {rover.status.devices.drive.stop_reason.reason}</p>
  {/if}
</div>

<style>
  .emergency-widget {
    min-height: 100%;
    display: grid;
    gap: 0.65rem;
    align-content: start;
  }
  /* Sized from the viewport height so the STOP button and its label fit short phone-landscape cells. */
  .estop {
    min-height: clamp(4.5rem, 40vh, 14rem);
    border: 1px solid var(--stopHover);
    border-radius: 0.55rem;
    background: var(--stop);
    color: white;
    cursor: pointer;
    font-family: inherit;
    letter-spacing: 0.04em;
    box-shadow: inset 0 -3px 0 rgba(0, 0, 0, 0.24);
  }
  .estop:hover {
    background: var(--stopHover);
  }
  .estop strong {
    display: block;
  }
  .estop strong {
    font-size: clamp(1.8rem, 9vh, 5.5rem);
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
  .reason {
    margin: 0;
    color: var(--textWarn);
    font-size: 0.75rem;
    overflow-wrap: anywhere;
  }
</style>
