<script lang="ts">
  import { rover } from "$lib/data/rover.svelte"
</script>

<span>{rover.battery.toFixed(0)}%</span>

<style>
  span {
    color: var(--warning);
    font-weight: 600;
  }
</style>
