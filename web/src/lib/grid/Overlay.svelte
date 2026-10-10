<script lang="ts">
  import { getContext } from "svelte"
  import type { Manager } from "./widgets.svelte"
  const manager = getContext("manager") as Manager

  let {
    x,
    y,
    isMouse = false,
  }: { x: number; y: number; isMouse?: boolean } = $props()

  function handlePointerMove() {
    if (manager.placeStart == null) return
    manager.updatePlace(x, y)
  }

  function handlePointerLeave() {
    cleanup()
    manager.resetPlace()
  }

  function handlePointerOut(e: PointerEvent) {
    var from = e.relatedTarget as Node | null
    if (!from || from.nodeName == "HTML") {
      handlePointerLeave()
    }
  }

  function cleanup() {
    window.removeEventListener("pointerout", handlePointerOut)
    window.removeEventListener("blur", handlePointerLeave)
  }
</script>

{#if isMouse}
  <button
    oncontextmenu={(e) => e.preventDefault()}
    onpointerdown={(e) => {
      if (e.button != 0 || !e.isPrimary) return
      manager.startPlace(x, y)

      window.addEventListener("pointerout", handlePointerOut)
      window.addEventListener("blur", handlePointerLeave)
    }}
    onpointerup={(e) => {
      if (e.button != 0 || !e.isPrimary) return
      manager.endPlace(x, y)
      cleanup()
    }}
    onpointercancel={handlePointerLeave}
    onpointermove={handlePointerMove}
    style="--x:{x};--y:{y};">+</button
  >
{:else}
  <div style="--x:{x};--y:{y};"></div>
{/if}

<style>
  div,
  button {
    position: absolute;
    background-color: transparent;
    color: inherit;
    border: 1px solid var(--bgLight);
    top: calc(var(--y) * var(--height));
    left: calc(var(--x) * var(--width));
    height: calc(var(--height));
    width: calc(var(--width));
    display: grid;
    place-content: center;
    touch-action: none;
  }
</style>
