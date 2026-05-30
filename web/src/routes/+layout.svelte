<script lang="ts">
  import { onMount, setContext, type Snippet } from "svelte"
  import Topbar from "$lib/Topbar.svelte"
  import "./global.css"

  import NotificationsUi from "$lib/NotificationsUI.svelte"
  import { manager } from "$lib/grid/widgets.svelte"
  import { STATUS_RATE_MS, UI_RATE_MS } from "$lib/constants"
  import { rover } from "$lib/rover-control.svelte"
  import { global } from "$lib"
  import CanvasRender from "$lib/grid/CanvasRender.svelte"

  setContext("manager", manager.manager)

  onMount(() => {
    rover.refreshStatus()
    const controlTimer = setInterval(rover.sendControlTick, UI_RATE_MS)
    const statusTimer = setInterval(rover.refreshStatus, STATUS_RATE_MS)
    const release = () => rover.releaseJoystick()
    const releaseWhenHidden = () => {
      if (document.hidden) rover.releaseJoystick()
    }

    window.addEventListener("blur", release)
    document.addEventListener("visibilitychange", releaseWhenHidden)

    return () => {
      clearInterval(controlTimer)
      clearInterval(statusTimer)
      window.removeEventListener("blur", release)
      document.removeEventListener("visibilitychange", releaseWhenHidden)
      rover.stopRover()
    }
  })

  let { children }: { children?: Snippet } = $props()
</script>

<NotificationsUi />
<section>
  <Topbar />

  {@render children?.()}

  <div class="preview-cache" aria-hidden="true">
    {#each global.allTemplates as t}
      <CanvasRender {t} show={false} />
    {/each}
  </div>
</section>

<style>
  section {
    overflow-y: auto;
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .preview-cache {
    display: contents;
    pointer-events: none;
  }
</style>
