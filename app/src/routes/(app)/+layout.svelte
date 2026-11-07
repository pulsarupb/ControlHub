<script lang="ts">
  import { onMount, setContext, untrack, type Snippet } from "svelte"
  import Topbar from "$lib/Topbar.svelte"
  import MotionBanner from "$lib/rover/MotionBanner.svelte"
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
    // Losing focus must stop both motion devices, not just release the joystick.
    const stopForFocusLoss = (reason: string) => {
      releaseGamepad()
      void rover.stopRover(reason)
    }
    const handleBlur = () => stopForFocusLoss("window lost focus")
    const handleVisibility = () => {
      if (document.hidden) stopForFocusLoss("app hidden")
    }

    window.addEventListener("blur", handleBlur)
    document.addEventListener("visibilitychange", handleVisibility)

    startGamepad()

    return () => {
      clearInterval(controlTimer)
      clearInterval(statusTimer)
      clearInterval(armTimer)
      window.removeEventListener("blur", handleBlur)
      document.removeEventListener("visibilitychange", handleVisibility)
      stopGamepad()
      void rover.stopRover("client closed")
    }
  })

  let { children }: { children?: Snippet } = $props()
</script>

<section>
  <Topbar />
  <MotionBanner />

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
