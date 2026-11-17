<script lang="ts">
  import type { Snippet } from "svelte"

  // One card for every "pick by looking" list: navlets, templates, empty-slot choosers.
  let {
    name,
    detail,
    frame = "auto",
    lazy = false,
    onclick,
    children,
  }: {
    name: string
    detail?: string
    frame?: string
    /** Mount the preview content only once the card is scrolled into view. */
    lazy?: boolean
    onclick: () => void
    children?: Snippet
  } = $props()

  let visible = $state(false)

  function watch(node: HTMLElement) {
    if (!lazy) {
      visible = true
      return
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          visible = true
          observer.disconnect()
        }
      },
      { rootMargin: "120px" },
    )
    observer.observe(node)
    return { destroy: () => observer.disconnect() }
  }
</script>

<button class="card" type="button" {onclick}>
  <span class="frame" style:height={frame} use:watch>
    {#if visible}{@render children?.()}{/if}
  </span>
  <span class="text">
    <span class="name">{name}</span>
    {#if detail}<span class="detail">{detail}</span>{/if}
  </span>
</button>

<style>
  .card {
    all: unset;
    box-sizing: border-box;
    display: grid;
    gap: 0.45rem;
    min-width: 0;
    padding: 0.4rem;
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    background: var(--bgDark);
    cursor: pointer;
  }
  .card:hover,
  .card:focus-visible {
    border-color: var(--borderStrong);
    background: var(--bgLight);
  }
  .card:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }
  .frame {
    display: grid;
    place-items: center;
    min-width: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 0.35rem;
    background: var(--bgMedium);
    pointer-events: none;
  }
  .text {
    display: grid;
    gap: 0.1rem;
    min-width: 0;
    padding: 0 0.15rem;
  }
  .name {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .detail {
    font-size: 0.75rem;
    color: var(--textMuted);
    font-variant-numeric: tabular-nums;
  }
</style>
