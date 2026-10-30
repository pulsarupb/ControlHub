<script lang="ts">
  import { onMount, setContext, untrack, type Snippet } from "svelte"
  import Topbar from "$lib/Topbar.svelte"
  import { manager } from "$lib/grid/widgets.svelte"
  import { STATUS_RATE_MS, UI_RATE_MS } from "$lib/constants"
  import { rover } from "$lib/rover-control.svelte"
  import { startGamepad, stopGamepad } from "$lib/gamepad-control.svelte"
  import { releaseGamepad } from "$lib/gamepad-control.svelte"
  import { armTick, releaseArm, loadHardwareConfig } from "$lib/hardware.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"

  setContext("manager", manager.manager)

  $effect(() => {
    roverConnection.baseUrl
    untrack(() => { rover.releaseJoystick(); releaseArm(); releaseGamepad() })
  })
  $effect(() => {
    if (roverConnection.state === "online") untrack(() => { void loadHardwareConfig() })
  })

  onMount(() => {
    rover.refreshStatus()
    const controlTimer = setInterval(rover.sendControlTick, UI_RATE_MS)
    const statusTimer = setInterval(rover.refreshStatus, STATUS_RATE_MS)
    const armTimer = setInterval(armTick, 100)
    const release = () => { rover.releaseJoystick(); releaseArm(); releaseGamepad() }
    const releaseWhenHidden = () => {
      if (document.hidden) release()
    }

    window.addEventListener("blur", release)
    document.addEventListener("visibilitychange", releaseWhenHidden)

    startGamepad()

    return () => {
      clearInterval(controlTimer)
      clearInterval(statusTimer)
      clearInterval(armTimer)
      window.removeEventListener("blur", release)
      document.removeEventListener("visibilitychange", releaseWhenHidden)
      stopGamepad()
      rover.stopRover()
    }
  })

  let { children }: { children?: Snippet } = $props()
</script>

<section>
  <Topbar />

  {@render children?.()}
</section>

<style>
  section {
    overflow-y: auto;
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
</style>
