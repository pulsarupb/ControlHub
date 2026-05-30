<script lang="ts">
  import { getComponent } from "$lib/widgets/registry"

  let {
    widgetID,
  }: {
    widgetID: string
  } = $props()

  const entry = $derived(getComponent(widgetID, "widget"))
</script>

{#if entry}
  <entry.component />
{:else}
  <p>Unknown widget: {widgetID}</p>
{/if}
