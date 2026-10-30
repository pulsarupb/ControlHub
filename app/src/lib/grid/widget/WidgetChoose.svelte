<script lang="ts">
  import { global } from "$lib"
  import Button from "$lib/ui/Button.svelte"
  import Overlay from "$lib/ui/Overlay.svelte"
  import { getComponents } from "$lib/widgets/registry"
  import PreviewBox from "../PreviewBox.svelte"
  import WidgetContent from "../WidgetContent.svelte"
  import { getContext } from "svelte"
  import type { Manager } from "../widgets.svelte"
  const manager = getContext("manager") as Manager

  let { set }: { set: (wID: string) => void } = $props()
</script>

<Overlay>
  {#snippet trigger()}
    <Button>Choose</Button>
  {/snippet}
  {#snippet overlay({ close }: { close: () => void })}
    <div class="possibilities">
      {#each getComponents("widget") as w}
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
            <PreviewBox scale={0.5}>
              <WidgetContent widgetID={w.id} />
            </PreviewBox>
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
  }
</style>
