<script lang="ts">
  import "virtual-gamepad-lib/gamepad_assets/base.css"
  import GAMEPAD_SVG from "virtual-gamepad-lib/gamepad_assets/rounded/display-gamepad-full.svg?raw"
  import { rover } from "$lib/data/rover.svelte"
  import { xboxButtonMap, xboxAxesMap } from "virtual-gamepad-lib"

  type ButtonState = {
    active: boolean
    value: number
  }

  const buttonElements: Record<number, string> = {
    [xboxButtonMap.A]: "button_1",
    [xboxButtonMap.B]: "button_2",
    [xboxButtonMap.X]: "button_3",
    [xboxButtonMap.Y]: "button_4",
    [xboxButtonMap.LShoulder]: "shoulder_button_front_left",
    [xboxButtonMap.RShoulder]: "shoulder_button_front_right",
    [xboxButtonMap.LTrigger]: "shoulder_trigger_back_left",
    [xboxButtonMap.RTrigger]: "shoulder_trigger_back_right",
    [xboxButtonMap.Back]: "select_button",
    [xboxButtonMap.Start]: "start_button",
    [xboxButtonMap.LStick]: "stick_button_left",
    [xboxButtonMap.RStick]: "stick_button_right",
    [xboxButtonMap.DPadUp]: "dpad_up",
    [xboxButtonMap.DPadDown]: "dpad_down",
    [xboxButtonMap.DPadLeft]: "dpad_left",
    [xboxButtonMap.DPadRight]: "dpad_right",
  }

  let gamepadEl: HTMLDivElement | undefined = $state()

  const buttons = $derived.by<ButtonState[]>(() => {
    const list = rover.gamepadButtons
    return Array.from({ length: 17 }, (_, index) => {
      const value = list[index] ? 1 : 0
      return {
        active: rover.gamepadConnected && value > 0,
        value,
      }
    })
  })

  const leftStick = $derived({
    x: axisValue(xboxAxesMap.LStickX),
    y: axisValue(xboxAxesMap.LStickY),
  })
  const rightStick = $derived({
    x: axisValue(xboxAxesMap.RStickX),
    y: axisValue(xboxAxesMap.RStickY),
  })

  function axisValue(index: number) {
    return rover.gamepadConnected ? (rover.gamepadAxes[index] ?? 0) : 0
  }

  function setActive(id: string, active: boolean, value = active ? 1 : 0) {
    const element = gamepadEl?.querySelector<SVGElement>(`#${id}`)
    if (!element) return

    element.classList.toggle("is-active", active)
    element.style.setProperty("--button-value", value.toString())
  }

  function setStick(id: string, x: number, y: number) {
    const element = gamepadEl?.querySelector<SVGElement>(`#${id}`)
    if (!element) return

    const range = 12
    element.style.transform = `translate(${x * range}px, ${y * range}px)`
  }

  function updateDisplay() {
    if (!gamepadEl) return

    for (const [index, id] of Object.entries(buttonElements)) {
      const state = buttons[Number(index)]
      setActive(id, state?.active ?? false, state?.value ?? 0)
    }

    setActive("stick_button_left", buttons[xboxButtonMap.LStick]?.active ?? false)
    setActive("stick_button_right", buttons[xboxButtonMap.RStick]?.active ?? false)
    setStick("stick_button_left", leftStick.x, leftStick.y)
    setStick("stick_button_right", rightStick.x, rightStick.y)
  }

  $effect(() => {
    buttons
    leftStick
    rightStick
    updateDisplay()
  })
</script>

<section class="gamepad" class:connected={rover.gamepadConnected}>
  <div class="gamepad-shell" bind:this={gamepadEl} aria-label="Xbox controller state">
    {@html GAMEPAD_SVG}
  </div>
</section>

<style>
  .gamepad {
    min-height: 100%;
    display: grid;
    overflow: hidden;
  }

  .gamepad-shell {
    min-height: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
  }

  .gamepad-shell :global(svg) {
    width: min(100%, 560px);
    height: 100%;
    max-height: 100%;
    display: block;
  }

  .gamepad:not(.connected) .gamepad-shell :global(svg) {
    filter: grayscale(1);
    opacity: 0.34;
  }

  .gamepad-shell :global(.gpad-base) {
    fill: var(--surfaceRaised) !important;
    stroke: var(--borderStrong) !important;
  }

  .gamepad-shell :global(.gpad-stick-base),
  .gamepad-shell :global(.gpad-btn-bg) {
    fill: var(--bgMedium) !important;
    stroke: var(--borderStrong) !important;
  }

  .gamepad-shell :global(.gpad-thumbstick) {
    transform-box: fill-box;
    transform-origin: center;
    transition: transform 80ms ease-out;
  }

  .gamepad-shell :global(.gpad-shadow),
  .gamepad-shell :global(.gpad-highlight),
  .gamepad-shell :global(.gpad-direction-highlight) {
    display: none;
  }

  .gamepad-shell :global(.gpad-btn-icon) {
    stroke: var(--textMuted) !important;
    fill: none !important;
  }

  .gamepad-shell :global(.gpad-btn-icon > path),
  .gamepad-shell :global(.gpad-btn-icon > circle) {
    stroke: inherit !important;
    fill: inherit !important;
  }

  .gamepad-shell :global(text) {
    display: none;
  }

  .gamepad-shell :global(.is-active .gpad-btn-bg),
  .gamepad-shell :global(.is-active.gpad-thumbstick .gpad-btn-bg),
  .gamepad-shell :global(.is-active.gpad-thumbstick circle) {
    fill: var(--accent) !important;
    stroke: color-mix(in srgb, var(--accent) 75%, white 25%) !important;
  }

  .gamepad-shell :global(.is-active .gpad-btn-icon) {
    stroke: var(--bgDark) !important;
  }

  .gamepad-shell :global(.is-active .gpad-btn-icon > path),
  .gamepad-shell :global(.is-active .gpad-btn-icon > circle) {
    stroke: inherit !important;
  }

  .gamepad-shell :global(#shoulder_trigger_back_left.is-active .gpad-btn-bg),
  .gamepad-shell :global(#shoulder_trigger_back_right.is-active .gpad-btn-bg) {
    translate: 0 calc(var(--button-value) * 3px);
  }
</style>
