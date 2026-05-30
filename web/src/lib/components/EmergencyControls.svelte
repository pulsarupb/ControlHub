<script lang="ts">
  let { onStop, onReset }: { onStop: () => void | Promise<void>; onReset: () => void | Promise<void> } = $props();
</script>

<article class="control-card danger-card">
  <button class="estop" onclick={onStop} aria-label="Emergency stop all rover motors">
    <span>WASTED?</span>
    <strong>STOP</strong>
  </button>
  <button class="reset" onclick={onReset}>Reset Everything</button>
  <p>Reset clears the stop, zeroes commands, and drops a fresh minimap spawn point.</p>
</article>

<style>
  .control-card {
    min-height: 0;
    border: 2px solid rgba(255, 255, 255, 0.28);
    border-radius: 8px;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.04)), rgba(8, 18, 28, 0.78);
    box-shadow:
      0 18px 48px rgba(0, 0, 0, 0.42),
      inset 0 0 0 1px rgba(0, 0, 0, 0.42);
    padding: 0.8rem;
    backdrop-filter: blur(10px) saturate(1.2);
  }

  .danger-card {
    display: grid;
    gap: 0.65rem;
  }

  .estop {
    min-height: clamp(9.5rem, 24vh, 14rem);
    border: 0;
    border-radius: 12px;
    background:
      linear-gradient(90deg, rgba(255, 255, 255, 0.08) 50%, transparent 50%) 0 0 / 18px 18px,
      radial-gradient(circle at 28% 20%, rgba(255, 255, 255, 0.38), transparent 22%),
      linear-gradient(145deg, #ff2d18, #8c0308 78%);
    color: white;
    cursor: pointer;
    font-family: Impact, Haettenschweiler, "Arial Narrow Bold", sans-serif;
    letter-spacing: 0.05em;
    text-shadow:
      0 5px 0 rgba(0, 0, 0, 0.42),
      0 0 18px rgba(255, 255, 255, 0.2);
    box-shadow:
      inset 0 -10px 0 rgba(0, 0, 0, 0.26),
      0 0 0 4px rgba(255, 255, 255, 0.16),
      0 18px 40px rgba(255, 33, 54, 0.38);
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
    font-size: clamp(1.35rem, 3.8vw, 2.45rem);
  }

  .estop strong {
    font-size: clamp(3.2rem, 8.8vw, 6.1rem);
    line-height: 0.8;
  }

  .reset {
    border: 2px solid rgba(255, 207, 79, 0.8);
    border-radius: 8px;
    background: linear-gradient(180deg, #ffe36c, #d09318);
    color: #161008;
    cursor: pointer;
    font-family: "Courier New", monospace;
    font-size: 1rem;
    font-weight: 700;
    padding: 0.75rem;
    text-transform: uppercase;
    box-shadow:
      inset 0 -4px 0 rgba(0, 0, 0, 0.22),
      0 0 24px rgba(255, 207, 79, 0.22);
  }

  p {
    margin: 0;
    color: #b9d3d0;
    font-family: "Courier New", monospace;
    font-size: 0.9rem;
    line-height: 1.45;
  }

  @media (max-width: 720px) {
    .estop {
      min-height: 10rem;
    }
  }
</style>
