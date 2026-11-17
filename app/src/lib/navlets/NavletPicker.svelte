<script lang="ts">
  import IconPlusRegular from "phosphor-icons-svelte/IconPlusRegular.svelte"
  import Overlay from "$lib/ui/Overlay.svelte"
  import NavletList from "./NavletList.svelte"

  let { onpick }: { onpick: (id: string) => void } = $props()
</script>

<Overlay>
  {#snippet trigger()}
    <span class="add" role="img" aria-label="Add navlet"><IconPlusRegular /></span>
  {/snippet}
  {#snippet overlay({ close }: { close: () => void })}
    <NavletList {onpick} {close} />
  {/snippet}
</Overlay>

<style>
  .add {
    display: grid;
    place-items: center;
    width: 1.8rem;
    height: 1.8rem;
    border-radius: 0.4rem;
    color: var(--textSubtle);
    cursor: pointer;
  }
  .add:hover {
    background: var(--bgLight);
    color: var(--text);
  }
  .add:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }
</style>
