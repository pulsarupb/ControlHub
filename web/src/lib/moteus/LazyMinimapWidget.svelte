<script lang="ts">
  import { onMount, type Component } from "svelte"

  let Minimap = $state<Component | null>(null)
  let failed = $state(false)

  onMount(() => {
    let active = true

    import("./MinimapWidget.svelte")
      .then((module) => {
        if (active) Minimap = module.default
      })
      .catch((error) => {
        console.error("Unable to load the minimap:", error)
        if (active) failed = true
      })

    return () => {
      active = false
    }
  })
</script>

{#if Minimap}
  <Minimap />
{:else}
  <div class="loading" role="status">
    {failed ? "Minimap unavailable" : "Loading minimap…"}
  </div>
{/if}

<style>
  .loading {
    width: 100%;
    height: 100%;
    min-height: 5rem;
    display: grid;
    place-items: center;
    color: var(--textMuted);
    background: var(--bgDark);
    font-size: 0.85rem;
  }
</style>
