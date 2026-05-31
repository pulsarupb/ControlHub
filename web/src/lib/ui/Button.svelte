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
    border-radius: 0.4rem;
    padding: 0.5em 0.85em;
    background: var(--surfaceRaised);
    color: var(--text);
    border: 1px solid var(--border);
    text-align: center;
    font-weight: 500;
  }
  button.selected,
  button:hover:not(:disabled) {
    border-color: var(--borderStrong);
    background: var(--bgLight);
  }
  button.transparent {
    background: transparent;
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
</style>
