<script lang="ts">
  import Portal from "svelte-portal"
  import type { Snippet } from "svelte"

  let {
    show = false,
    title = "",
    width = "min(680px, calc(100vw - 32px))",
    children,
    onclose,
  }: {
    show?: boolean
    title?: string
    width?: string
    children?: Snippet
    onclose?: () => void
  } = $props()

  function handleBackdropClick() {
    onclose?.()
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onclose?.()
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if show}
  <Portal>
    <div class="backdrop" onclick={handleBackdropClick} role="presentation"></div>
    <div class="modal" style="--width: {width}" role="dialog" aria-label={title || "Dialog"}>
      <div class="header">
        <h2>{title}</h2>
        <button class="close-btn" onclick={handleBackdropClick} aria-label="Close">
          ✕
        </button>
      </div>
      <div class="content">
        {@render children?.()}
      </div>
    </div>
  </Portal>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.55);
  }
  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 1001;
    background: var(--bgMedium);
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    width: var(--width);
    max-height: calc(100vh - 32px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.85rem 1.2rem;
    border-bottom: 1px solid var(--border);
    background: var(--bgDark);
  }
  .header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }
  .close-btn {
    all: unset;
    cursor: pointer;
    font-size: 1.2rem;
    color: var(--textMuted);
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 0.35rem;
  }
  .close-btn:hover {
    color: var(--text);
    background: var(--bgLight);
  }
  .content {
    flex: 1;
    overflow: auto;
    display: flex;
  }
</style>
