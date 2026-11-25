<script lang="ts">
  import { onMount, setContext, untrack, type Snippet } from "svelte"
  import Topbar from "$lib/Topbar.svelte"
  import MotionBanner from "$lib/rover/MotionBanner.svelte"
  import { manager } from "$lib/grid/widgets.svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { startGamepad, stopGamepad } from "$lib/gamepad-control.svelte"
  import { releaseGamepad } from "$lib/gamepad-control.svelte"
  import { releaseArm, loadHardwareConfig } from "$lib/hardware.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"

  setContext("manager", manager.manager)

  let mounted = false

  $effect(() => {
    const baseUrl = roverConnection.baseUrl
    untrack(() => {
      rover.releaseJoystick()
      releaseArm()
      releaseGamepad()
      if (mounted) rover.startChannel(baseUrl)
    })
  })
  $effect(() => {
    if (roverConnection.state === "online") untrack(() => { void loadHardwareConfig() })
  })

  onMount(() => {
    mounted = true
    rover.startChannel(roverConnection.baseUrl)
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
      mounted = false
      window.removeEventListener("blur", handleBlur)
      document.removeEventListener("visibilitychange", handleVisibility)
      stopGamepad()
      void rover.stopRover("client closed").finally(() => rover.stopChannel())
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
