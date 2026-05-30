<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
</script>

<div class="emergency-widget">
  <button class="estop" onclick={rover.stopRover} aria-label="Emergency stop all rover motors">
    <span>Emergency</span>
    <strong>STOP</strong>
  </button>
  <button class="reset" onclick={rover.resetRover}>Reset Rover</button>
  <p>Reset clears the stop, zeroes commands, and drops a fresh minimap spawn point.</p>
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
    border: 0;
    border-radius: 0.9rem;
    background:
      linear-gradient(90deg, rgba(255, 255, 255, 0.08) 50%, transparent 50%) 0 0 / 18px 18px,
      radial-gradient(circle at 28% 20%, rgba(255, 255, 255, 0.38), transparent 22%),
      linear-gradient(145deg, #ff2d18, #8c0308 78%);
    color: white;
    cursor: pointer;
    font-family: Impact, Haettenschweiler, "Arial Narrow Bold", sans-serif;
    letter-spacing: 0.05em;
    text-shadow: 0 5px 0 rgba(0, 0, 0, 0.42), 0 0 18px rgba(255, 255, 255, 0.2);
    box-shadow: inset 0 -10px 0 rgba(0, 0, 0, 0.26), 0 0 0 4px rgba(255, 255, 255, 0.12), 0 18px 40px rgba(255, 33, 54, 0.28);
    transform: skew(-2deg);
  }
  .estop:hover {
    filter: saturate(1.18) brightness(1.06);
  }
  .estop span,
  .estop strong {
    display: block;
  }
  .estop span {
    font-size: clamp(1.15rem, 3vw, 2.2rem);
  }
  .estop strong {
    font-size: clamp(3rem, 8vw, 6rem);
    line-height: 0.8;
  }
  .reset {
    border: 1px solid rgba(249, 115, 22, 0.85);
    border-radius: 0.75rem;
    background: linear-gradient(180deg, #facc15, #f97316);
    color: #161008;
    cursor: pointer;
    font: inherit;
    font-weight: 700;
    padding: 0.8rem;
    text-transform: uppercase;
    box-shadow: inset 0 -4px 0 rgba(0, 0, 0, 0.22), 0 0 24px rgba(249, 115, 22, 0.22);
  }
  p {
    margin: 0;
    color: rgba(255, 255, 255, 0.72);
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
    font-size: 0.9rem;
    line-height: 1.45;
  }
</style>
