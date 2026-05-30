<script lang="ts">
  import { getComponent } from "$lib/widgets/registry"

  let {
    widgetID,
  }: {
    widgetID: string
  } = $props()

  const entry = $derived(getComponent(widgetID, "navlet"))
</script>

{#if entry}
  <entry.component />
{/if}
