<script lang="ts">
  import PreviewCard from "$lib/ui/PreviewCard.svelte"
  import { getComponents } from "$lib/widgets/registry"
  import NavletContent from "./NavletContent.svelte"

  // Shared by "+" (add) and "Choose" (fill an empty slot) so both show the same previews.
  let {
    onpick,
    close,
  }: {
    onpick: (id: string) => void
    close: () => void
  } = $props()
</script>

<div class="list">
  {#each getComponents("navlet") as w (w.id)}
    <PreviewCard
      name={w.name}
      frame="64px"
      onclick={() => {
        close()
        onpick(w.id)
      }}
    >
      <NavletContent widgetID={w.id} />
    </PreviewCard>
  {/each}
</div>

<style>
  .list {
    width: min(360px, calc(100vw - 32px));
    max-height: min(70vh, 520px);
    overflow-y: auto;
    padding: 0.5rem;
    display: grid;
    gap: 0.5rem;
  }
</style>
