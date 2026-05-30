<script lang="ts">
  import { tick } from "svelte"
  import Portal from "svelte-portal"
  import type { Snippet } from "svelte"

  let {
    trigger,
    overlay,
    triggerStyle = "",
    onStateChange,
  }: {
    trigger?: Snippet<[{ isOpen: boolean }]>
    overlay?: Snippet<[{ close: () => void }]>
    triggerStyle?: string
    onStateChange?: (isOpen: boolean) => void
  } = $props()

  let isOpen = $state(false)
  let triggerElement = $state<HTMLButtonElement>()
  let panelElement = $state<HTMLDivElement>()
  let panelLeft = $state(0)
  let panelTop = $state(0)

  const viewportPadding = 8

  async function setOpen(value: boolean) {
    isOpen = value
    onStateChange?.(isOpen)
    if (isOpen) {
      await tick()
      updatePosition()
      window.addEventListener("resize", updatePosition)
      window.addEventListener("scroll", updatePosition, true)
    } else {
      removePositionListeners()
    }
  }

  function close() {
    setOpen(false)
  }

  function removePositionListeners() {
    if (typeof window === "undefined") return
    window.removeEventListener("resize", updatePosition)
    window.removeEventListener("scroll", updatePosition, true)
  }

  function updatePosition() {
    if (!triggerElement || !panelElement) return

    const triggerRect = triggerElement.getBoundingClientRect()
    const panelRect = panelElement.getBoundingClientRect()

    const preferredLeft = triggerRect.right - panelRect.width
    const preferredTop = triggerRect.bottom + 8

    panelLeft = Math.min(
      Math.max(preferredLeft, viewportPadding),
      window.innerWidth - panelRect.width - viewportPadding
    )
    panelTop = Math.min(
      Math.max(preferredTop, viewportPadding),
      window.innerHeight - panelRect.height - viewportPadding
    )
  }

  $effect(() => {
    return () => removePositionListeners()
  })
</script>

<span class="overlay-root">
  <button
    bind:this={triggerElement}
    class="trigger"
    style={triggerStyle}
    onclick={() => setOpen(!isOpen)}
  >
    {@render trigger?.({ isOpen })}
  </button>
  {#if isOpen}
    <Portal>
      <button class="backdrop" aria-label="Close overlay" onclick={close}></button>
      <div
        bind:this={panelElement}
        class="panel"
        style="--left:{panelLeft}px;--top:{panelTop}px;"
      >
        {@render overlay?.({ close })}
      </div>
    </Portal>
  {/if}
</span>

<style>
  .overlay-root {
    position: relative;
    display: inline-flex;
  }
  .trigger {
    all: unset;
    cursor: pointer;
  }
  .backdrop {
    all: unset;
    position: fixed;
    inset: 0;
    z-index: 1000;
  }
  .panel {
    position: fixed;
    left: var(--left);
    top: var(--top);
    z-index: 1001;
    background: var(--bgMedium);
    border: 1px solid var(--bgLight);
    border-radius: 1rem;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.35);
    max-width: calc(100vw - 16px);
    max-height: calc(100vh - 16px);
    overflow: auto;
  }
</style>
