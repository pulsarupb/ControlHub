<script lang="ts">
  import Button from "$lib/ui/Button.svelte"
  import Overlay from "$lib/ui/Overlay.svelte"
  import CanvasRender from "./CanvasRender.svelte"
  import type { Template } from "$lib/types"
  import { global } from "$lib"

  let { set }: { set: (t: Template) => void } = $props()
</script>

<Overlay triggerStyle="width: 100%;min-width: 0;">
  {#snippet trigger()}
    <Button style="width: 100%;box-sizing: border-box;">Templates</Button>
  {/snippet}
  {#snippet overlay({ close }: { close: () => void })}
    <div class="possibilities">
      {#each global.allTemplates as t}
        <button
          class="choose"
          onclick={() => {
            close()
            set(t)
          }}
        >
          <p>Local</p>
          <h4>{t.name}</h4>
          <CanvasRender {t} />
        </button>
      {/each}
    </div>
  {/snippet}
</Overlay>

<style>
  h4,
  p {
    margin: 0;
    text-align: center;
  }
  h4 {
    margin-bottom: var(--padding);
  }
  .possibilities {
    width: min(750px, calc(100vw - 32px));
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, auto));
    gap: 1rem;
    padding: var(--padding);
  }
  button.choose {
    all: unset;
    cursor: pointer;
    background-color: var(--bgLight);
    padding: 0.5em;
    border-radius: 0.8rem;
  }
</style>
