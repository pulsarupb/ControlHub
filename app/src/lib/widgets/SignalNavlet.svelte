<script lang="ts">
  import { rover } from "$lib/data/rover.svelte"
</script>

<span>Signal {rover.signal.toFixed(0)}%</span>

<style>
  span {
    color: var(--accent);
    font-weight: 600;
  }
</style>
