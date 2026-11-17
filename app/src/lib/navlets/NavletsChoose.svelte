<script lang="ts">
  import Overlay from "$lib/ui/Overlay.svelte"
  import { getContext } from "svelte"
  import type { Manager } from "$lib/grid/widgets.svelte"
  import NavletList from "./NavletList.svelte"
  const manager = getContext("manager") as Manager

  let { set }: { set: (wID: string) => void } = $props()
</script>

<Overlay>
  {#snippet trigger()}
    <!-- A span, because the overlay trigger is already a button. -->
    <span class="choose-trigger">Choose</span>
  {/snippet}
  {#snippet overlay({ close }: { close: () => void })}
    <NavletList
      onpick={(id) => {
        set(id)
        manager.save()
      }}
      {close}
    />
  {/snippet}
</Overlay>

<style>
  .choose-trigger {
    display: inline-flex;
    align-items: center;
    padding: 0.3rem 0.7rem;
    border: 1px solid var(--borderStrong);
    border-radius: 0.4rem;
    background: var(--bgLight);
    color: var(--text);
    font-size: 0.8rem;
    cursor: pointer;
  }
</style>
