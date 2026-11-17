<script lang="ts">
  import Button from "$lib/ui/Button.svelte"
  import Overlay from "$lib/ui/Overlay.svelte"
  import PreviewCard from "$lib/ui/PreviewCard.svelte"
  import TemplateMiniature from "./TemplateMiniature.svelte"
  import type { Template } from "$lib/types"
  import { global } from "$lib"

  let { set }: { set: (t: Template) => void } = $props()
</script>

<Overlay triggerStyle="width: 100%;min-width: 0;">
  {#snippet trigger()}
    <Button style="width: 100%;box-sizing: border-box;">Templates</Button>
  {/snippet}
  {#snippet overlay({ close }: { close: () => void })}
    <div class="templates">
      {#each global.allTemplates as t}
        <PreviewCard
          name={t.name}
          detail="{t.widgets.length} groups · {t.navlets.length} navlets"
          onclick={() => {
            close()
            set(t)
          }}
        >
          <TemplateMiniature {t} />
        </PreviewCard>
      {/each}
    </div>
  {/snippet}
</Overlay>

<style>
  .templates {
    width: min(640px, calc(100vw - 32px));
    max-height: min(72vh, 620px);
    overflow-y: auto;
    padding: 0.5rem;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.6rem;
  }
</style>
