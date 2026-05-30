<script lang="ts">
  import { onMount } from "svelte";
  import EmergencyControls from "$lib/components/EmergencyControls.svelte";
  import JoystickControl from "$lib/components/JoystickControl.svelte";
  import Minimap from "$lib/components/Minimap.svelte";
  import StatusCard from "$lib/components/StatusCard.svelte";
  import TelemetryCard from "$lib/components/TelemetryCard.svelte";
  import { STATUS_RATE_MS, UI_RATE_MS } from "$lib/constants";
  import { roverControl } from "$lib/rover-control.svelte";

  const rover = roverControl();

  onMount(() => {
    rover.refreshStatus();
    const controlTimer = setInterval(rover.sendControlTick, UI_RATE_MS);
    const statusTimer = setInterval(rover.refreshStatus, STATUS_RATE_MS);
    const release = () => rover.releaseJoystick();
    const releaseWhenHidden = () => {
      if (document.hidden) rover.releaseJoystick();
    };

    window.addEventListener("blur", release);
    document.addEventListener("visibilitychange", releaseWhenHidden);

    return () => {
      clearInterval(controlTimer);
      clearInterval(statusTimer);
      window.removeEventListener("blur", release);
      document.removeEventListener("visibilitychange", releaseWhenHidden);
      rover.stopRover();
    };
  });
</script>

<svelte:head>
  <title>Rover Control</title>
  <meta name="description" content="Embedded rover control panel" />
</svelte:head>

<main class:stopped={rover.stopped}>
  <StatusCard connectionState={rover.connectionState} stopped={rover.stopped} />

  <section class="grid">
    <EmergencyControls onStop={rover.stopRover} onReset={rover.resetRover} />
    <JoystickControl
      throttle={rover.throttle}
      steering={rover.steering}
      joystick={rover.joystick}
      joystickActive={rover.joystickActive}
      startJoystick={rover.startJoystick}
      updateJoystick={rover.setJoystick}
      releaseJoystick={rover.releaseJoystick}
    />
    <Minimap pose={rover.pose} path={rover.path} />
    <TelemetryCard status={rover.status} />
  </section>
</main>

<style>
  main {
    height: 100vh;
    overflow: hidden;
    padding: clamp(0.7rem, 1.35vw, 1.25rem);
  }

  .grid {
    display: grid;
    grid-template-columns: minmax(240px, 0.8fr) minmax(260px, 0.9fr) minmax(340px, 1.7fr);
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 0.7rem;
    height: calc(100vh - clamp(0.7rem, 1.35vw, 1.25rem) * 2 - 8.4rem);
    min-height: 0;
  }

  main.stopped :global(.joystick) {
    opacity: 0.58;
  }

  @media (max-width: 1100px) {
    .grid {
      grid-template-columns: 1fr 1fr;
      height: auto;
      overflow: visible;
    }
  }

  @media (max-width: 720px) {
    main {
      height: auto;
      min-height: 100vh;
      overflow: visible;
    }

    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
