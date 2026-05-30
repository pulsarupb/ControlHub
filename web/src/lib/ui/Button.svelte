<script lang="ts">
  import type { Snippet } from "svelte"

  let {
    children,
    selected = false,
    transparent = false,
    disabled = false,
    style = "",
    onclick,
  }: {
    children?: Snippet
    selected?: boolean
    transparent?: boolean
    disabled?: boolean
    style?: string
    onclick?: (event: MouseEvent) => void
  } = $props()
</script>

<button {disabled} {style} class:selected class:transparent onclick={onclick}>
  {@render children?.()}
</button>

<style>
  button {
    all: unset;
    cursor: pointer;
    border-radius: 0.6rem;
    padding: 0.45em 0.85em;
    background: var(--bgLight);
    color: var(--text);
    border: 1px solid transparent;
    text-align: center;
  }
  button.selected,
  button:hover:not(:disabled) {
    border-color: var(--primary);
  }
  button.transparent {
    background: transparent;
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
</style>
