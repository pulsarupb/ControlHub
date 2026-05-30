<script lang="ts">
  import Options from "$lib/icons/Options.svelte"
  import Button from "$lib/ui/Button.svelte"
  import Overlay from "$lib/ui/Overlay.svelte"
  import NavletContent from "$lib/navlets/NavletContent.svelte"
  import NavletsChoose from "./NavletsChoose.svelte"
  import { setContext } from "svelte"
  import type { Manager } from "$lib/grid/widgets.svelte"
  import Add from "$lib/icons/Add.svelte"
  import Delete from "$lib/icons/Delete.svelte"

  let {
    manager = $bindable(),
  }: {
    manager: Manager
  } = $props()

  setContext("manager", manager)
</script>

<div class="main">
  {#each manager.navlets as navlet}
    {#if manager.isValidNavlet(navlet.navletID)}
      <div class="navlet">
        <NavletContent widgetID={navlet.navletID} />
      </div>
    {/if}
  {/each}

  <Overlay triggerStyle={"margin-top: 6px;"}>
    {#snippet trigger()}
      <div class="options">
        <Options />
      </div>
    {/snippet}
    {#snippet overlay({ close }: { close: () => void })}
      <div class="menu">
        {#each manager.navlets as navlet, index}
          {#if manager.isValidNavlet(navlet.navletID)}
            <Button>{navlet.navletID}</Button>

            <Button
              transparent={true}
              style="width: 36px;height: 36px;padding: 0;display: grid;place-items: center;"
              onclick={() => manager.removeNavlet(index)}
            >
              <span class="delete-icon"><Delete /></span>
            </Button>
          {:else}
            <NavletsChoose
              set={(wID) => {
                navlet.navletID = wID
              }}
            />
          {/if}
        {/each}

        <Button
          style="grid-column-start: span 2;"
          onclick={() => {
            manager.addNavlet()
          }}
        >
          <Add />
        </Button>
      </div>
    {/snippet}
  </Overlay>
</div>

<style>
  .main {
    display: flex;
    align-items: center;
    gap: var(--padding);
    height: 100%;
    flex-grow: 1;
  }
  .menu {
    display: grid;
    gap: calc(var(--padding) / 2);
    padding: calc(var(--padding) / 2);
    grid-template-columns: minmax(120px, auto) 36px;
  }
  .delete-icon {
    width: 18px;
    height: 22px;
    display: grid;
    place-items: center;
  }
</style>
