<script lang="ts">
  import PreviewBox from "$lib/grid/PreviewBox.svelte"
  import Button from "$lib/ui/Button.svelte"
  import Overlay from "$lib/ui/Overlay.svelte"
  import { getComponents } from "$lib/widgets/registry"
  import NavletContent from "./NavletContent.svelte"
  import { getContext } from "svelte"
  import type { Manager } from "$lib/grid/widgets.svelte"
  const manager = getContext("manager") as Manager

  let { set }: { set: (wID: string) => void } = $props()
</script>

<Overlay>
  {#snippet trigger()}
    <Button>Choose</Button>
  {/snippet}
  {#snippet overlay({ close }: { close: () => void })}
    <div class="possibilities">
      {#each getComponents("navlet") as w}
          <button
            class="choose"
            onclick={() => {
              close()
              set(w.id)
              manager.save()
            }}
          >
            <p>Local</p>
            <h4>{w.name}</h4>
            <div class="navlet-preview">
              <PreviewBox scale={1} height="48px">
                <NavletContent widgetID={w.id} />
              </PreviewBox>
            </div>
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
    margin-bottom: calc(var(--padding) / 2);
  }
  .possibilities {
    width: 500px;
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
    display: grid;
    gap: 0.35rem;
  }
  .navlet-preview {
    display: grid;
    place-items: center;
    min-height: 56px;
  }
</style>
