<script lang="ts">
  import Overlay from "$lib/ui/Overlay.svelte"
  import IconPlusRegular from "phosphor-icons-svelte/IconPlusRegular.svelte"
  import WidgetList from "./WidgetList.svelte"

  // "+" in the group header: opens the widget picker straight away. The pick becomes a new tab.
  let { onpick }: { onpick: (id: string) => void } = $props()
</script>

<span class="add-root">
  <Overlay triggerStyle="display:grid;place-items:center;width:var(--ctl);height:var(--ctl);padding:0;border:0;border-radius:0.4rem;background:transparent;color:var(--textSubtle);cursor:pointer;">
    {#snippet trigger()}
      <span class="add-icon" role="img" aria-label="Add widget">
        <IconPlusRegular />
      </span>
    {/snippet}
    {#snippet overlay({ close }: { close: () => void })}
      <WidgetList {onpick} {close} />
    {/snippet}
  </Overlay>
</span>

<style>
  .add-root {
    flex: none;
    display: inline-flex;
  }
  :global(.add-root .trigger:hover) .add-icon {
    background: var(--bgLight);
    color: var(--text);
  }
  :global(.add-root .trigger:focus-visible) {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }
  .add-icon {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    border-radius: 0.4rem;
    font-size: 0.95rem;
  }
</style>
