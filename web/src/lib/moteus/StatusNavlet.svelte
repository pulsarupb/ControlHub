<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
</script>

<span class="pill" class:offline={rover.connectionState === "offline"} class:stopped={rover.stopped}>
  {rover.connectionState} / {rover.stopped ? "locked" : "ready"}
</span>

<style>
  .pill {
    border: 1px solid rgba(45, 212, 191, 0.4);
    border-radius: 99rem;
    padding: 0.35rem 0.75rem;
    background: rgba(45, 212, 191, 0.12);
    color: #ccfbf1;
    white-space: nowrap;
  }
  .pill.offline,
  .pill.stopped {
    border-color: rgba(249, 115, 22, 0.5);
    background: rgba(249, 115, 22, 0.14);
    color: #fed7aa;
  }
</style>
