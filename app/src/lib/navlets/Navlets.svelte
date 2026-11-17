<script lang="ts">
  import IconXRegular from "phosphor-icons-svelte/IconXRegular.svelte"
  import NavletContent from "$lib/navlets/NavletContent.svelte"
  import NavletsChoose from "./NavletsChoose.svelte"
  import NavletPicker from "./NavletPicker.svelte"
  import { confirmDelete } from "$lib/ui/confirm.svelte"
  import { setContext } from "svelte"
  import type { Manager } from "$lib/grid/widgets.svelte"

  let {
    manager = $bindable(),
  }: {
    manager: Manager
  } = $props()

  setContext("manager", manager)

  // Touch screens have no hover: tapping a navlet reveals its remove control, and tapping anywhere else hides it.
  let revealed: number | null = $state(null)
  function updateReveal(e: PointerEvent) {
    const navlet = (e.target as Element | null)?.closest?.("[data-navlet]")
    revealed = navlet ? Number((navlet as HTMLElement).dataset.navlet) : null
  }
</script>

<svelte:window onpointerdown={updateReveal} />

<div class="main">
  {#each manager.navlets as navlet, index}
    {#if manager.isValidNavlet(navlet.navletID)}
      <div class="navlet" data-navlet={index} class:revealed={revealed === index}>
        <NavletContent widgetID={navlet.navletID} />
        {#if manager.enableInteractions}
          <button class="remove" aria-label="Remove navlet" title="Remove navlet" onclick={() => confirmDelete("Remove navlet?", "This removes the navlet from the bar.", () => manager.removeNavlet(index))}>
            <IconXRegular />
          </button>
        {/if}
      </div>
    {:else}
      <!-- Empty slot: pick from the same preview list as the + button. -->
      <div class="navlet slot">
        <NavletsChoose set={(wID) => { navlet.navletID = wID; manager.save() }} />
        <button class="remove" aria-label="Remove empty slot" title="Remove empty slot" onclick={() => manager.removeNavlet(index)}>
          <IconXRegular />
        </button>
      </div>
    {/if}
  {/each}

  {#if manager.enableInteractions}
    <NavletPicker onpick={(id) => { manager.navlets.push({ navletID: id }); manager.save() }} />
  {/if}
</div>

<style>
  /* Wraps onto more rows on phones instead of scrolling sideways. */
  .main {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem 0.6rem;
    min-width: 0;
    min-height: 36px;
  }
  /* Wide screens (phone landscape included): the navlets stay on one row that scrolls inside the bar,
     so a bar with many navlets never grows taller than the top bar. Phones keep the wrapping row below. */
  @media (min-width: 521px) {
    .main {
      flex: 1 1 0;
      flex-wrap: nowrap;
      overflow-x: auto;
      overflow-y: hidden;
      scrollbar-width: none;
      padding: 0.5rem 0.25rem 0;
      margin: -0.5rem -0.25rem 0;
    }
    .main::-webkit-scrollbar { display: none; }
    .navlet { flex: none; }
  }
  .navlet {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
  }
  .remove {
    all: unset;
    box-sizing: border-box;
    position: absolute;
    top: -0.45rem;
    right: -0.45rem;
    display: none;
    place-items: center;
    width: 1.1rem;
    height: 1.1rem;
    border-radius: 999px;
    background: var(--bgLight);
    border: 1px solid var(--borderStrong);
    color: var(--textMuted);
    font-size: 0.6rem;
    cursor: pointer;
  }
  /* Hover reveals the remove control only where a pointer can hover. Empty slots always show theirs. */
  @media (hover: hover) {
    .navlet:hover .remove,
    .navlet:focus-within .remove { display: grid; }
  }
  .slot .remove { display: grid; }
  .remove:hover { color: var(--danger); border-color: var(--danger); }
  .remove:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }
  /* Touch screens: the remove control sits inline after the navlet, larger, and appears only after the navlet is tapped. */
  @media (pointer: coarse) {
    .remove {
      position: static;
      flex: none;
      margin-left: 0.25rem;
      width: 1.4rem;
      height: 1.4rem;
      font-size: 0.7rem;
    }
    .navlet.revealed .remove { display: grid; }
  }
</style>
