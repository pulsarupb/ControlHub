<script lang="ts">
  import { rover } from "$lib/data/rover.svelte"
</script>

<span class="pill">{rover.mode}</span>

<style>
  .pill {
    border: 1px solid var(--bgLight);
    border-radius: 99rem;
    padding: 0.35rem 0.75rem;
    background: var(--bgDark);
  }
</style>
