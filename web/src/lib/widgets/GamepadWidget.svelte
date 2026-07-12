<script lang="ts">
  import "virtual-gamepad-lib/gamepad_assets/base.css"
  import GAMEPAD_SVG from "virtual-gamepad-lib/gamepad_assets/rounded/display-gamepad-full.svg?raw"
  import { rover as displayRover } from "$lib/data/rover.svelte"
  import { rover as controlRover } from "$lib/rover-control.svelte"
  import { xboxButtonMap, xboxAxesMap } from "virtual-gamepad-lib"

  type DragState = {
    id: string
    centerX: number
    centerY: number
  }

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

  const buttonActions: Record<string, () => void> = {
    button_2: () => controlRover.stopRover(),
    shoulder_trigger_back_right: () => controlRover.stopRover(),
    select_button: () => controlRover.stopRover(),
    button_4: () => controlRover.resetRover(),
    shoulder_button_front_right: () => controlRover.resetRover(),
  }

  const STICK_RANGE = 12

  let gamepadEl: HTMLDivElement | undefined = $state()
  let dragState: DragState | null = null
  let pressedButton: string | null = null

  const buttons = $derived.by<ButtonState[]>(() => {
    const list = displayRover.gamepadButtons
    return Array.from({ length: 17 }, (_, index) => {
      const value = list[index] ? 1 : 0
      return {
        active: displayRover.gamepadConnected && value > 0,
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
    return displayRover.gamepadConnected ? (displayRover.gamepadAxes[index] ?? 0) : 0
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

    element.style.transform = `translate(${x * STICK_RANGE}px, ${y * STICK_RANGE}px)`
  }

  function updateDisplay() {
    if (!gamepadEl) return

    for (const [index, id] of Object.entries(buttonElements)) {
      const state = buttons[Number(index)]
      setActive(id, state?.active ?? false, state?.value ?? 0)
    }

    setActive("stick_button_left", buttons[xboxButtonMap.LStick]?.active ?? false)
    setActive("stick_button_right", buttons[xboxButtonMap.RStick]?.active ?? false)

    if (dragState?.id !== "stick_button_left") {
      setStick("stick_button_left", leftStick.x, leftStick.y)
    }
    if (dragState?.id !== "stick_button_right") {
      setStick("stick_button_right", rightStick.x, rightStick.y)
    }
  }

  $effect(() => {
    buttons
    leftStick
    rightStick
    updateDisplay()
  })

  let injected = false
  $effect(() => {
    if (!gamepadEl || injected) return
    injected = true

    const svg = gamepadEl.querySelector("svg")
    if (!svg) return

    const parent = svg.querySelector("#dpad_up")?.parentElement
    if (!parent) return

    const positions = [
      { id: "dpad_up_bg", cx: 77, cy: 79 },
      { id: "dpad_down_bg", cx: 77, cy: 117 },
      { id: "dpad_left_bg", cx: 58, cy: 98 },
      { id: "dpad_right_bg", cx: 97, cy: 98 },
    ]

    const ref = parent.querySelector("#dpad_right_button_icon")?.parentElement ?? null

    for (const { id, cx, cy } of positions) {
      if (parent.querySelector(`#${id}`)) continue
      const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle")
      circle.setAttribute("cx", String(cx))
      circle.setAttribute("cy", String(cy))
      circle.setAttribute("r", "11")
      circle.id = id
      circle.classList.add("gpad-dpad-bg")
      parent.insertBefore(circle, ref)
    }
  })

  function getSvgPoint(clientX: number, clientY: number): DOMPoint {
    const svg = gamepadEl!.querySelector<SVGSVGElement>("svg")!
    const pt = svg.createSVGPoint()
    pt.x = clientX
    pt.y = clientY
    return pt.matrixTransform(svg.getScreenCTM()!.inverse())
  }

  function elementSvgCenter(id: string): { x: number; y: number } | null {
    const el = gamepadEl!.querySelector<SVGElement>(`#${id}`)
    if (!el) return null

    const svg = gamepadEl!.querySelector("svg")!
    const rect = el.getBoundingClientRect()
    const pt = svg.createSVGPoint()
    pt.x = rect.left + rect.width / 2
    pt.y = rect.top + rect.height / 2
    return pt.matrixTransform(svg.getScreenCTM()!.inverse())
  }

  function onPointerDown(e: PointerEvent) {
    const target = e.target as SVGElement
    if (!target.id.endsWith("_tap_target")) return

    if (controlRover.stopped) return

    const buttonId = target.id.replace("_tap_target", "")

    if (buttonId === "stick_button_left" || buttonId === "stick_button_right") {

      e.preventDefault()
      target.setPointerCapture(e.pointerId)

      const stickEl = gamepadEl!.querySelector<SVGElement>(`#${buttonId}`)
      if (!stickEl) return

      const center = elementSvgCenter(buttonId)
      if (!center) return

      stickEl.style.transition = "none"
      dragState = { id: buttonId, centerX: center.x, centerY: center.y }
      controlRover.mouseJoystickActive = true
      controlRover.startJoystick()
      return
    }

    pressedButton = buttonId
    setActive(buttonId, true)
    if (buttonId.startsWith("dpad_")) {
      setActive(`${buttonId}_button_icon`, true)
      setActive(`${buttonId}_bg`, true)
    }
    buttonActions[buttonId]?.()
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragState) return

    const pt = getSvgPoint(e.clientX, e.clientY)
    const stickEl = gamepadEl!.querySelector<SVGElement>(`#${dragState.id}`)

    const dx = pt.x - dragState.centerX
    const dy = pt.y - dragState.centerY
    const clampedDx = Math.max(-STICK_RANGE, Math.min(STICK_RANGE, dx))
    const clampedDy = Math.max(-STICK_RANGE, Math.min(STICK_RANGE, dy))
    const nx = clampedDx / STICK_RANGE
    const ny = clampedDy / STICK_RANGE

    if (stickEl) {
      stickEl.style.transform = `translate(${clampedDx}px, ${clampedDy}px)`
    }

    if (!controlRover.stopped) {
      if (dragState.id === "stick_button_left") {
        controlRover.setThrottle(-ny)
      } else {
        controlRover.setSteering(nx)
      }
    }
  }

  function onPointerUp(_e: PointerEvent) {
    if (dragState) {
      const stickEl = gamepadEl?.querySelector<SVGElement>(`#${dragState.id}`)
      if (stickEl) {
        stickEl.style.transition = ""
        stickEl.style.transform = ""
      }
      controlRover.releaseJoystick()
      controlRover.mouseJoystickActive = false
      dragState = null
      return
    }

    if (pressedButton) {
      setActive(pressedButton, false)
      if (pressedButton.startsWith("dpad_")) {
        setActive(`${pressedButton}_button_icon`, false)
        setActive(`${pressedButton}_bg`, false)
      }
      pressedButton = null
    }
  }
</script>

<section
  class="gamepad"
  class:connected={displayRover.gamepadConnected}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointerleave={onPointerUp}
>
  <div class="gamepad-shell" bind:this={gamepadEl} aria-label="Xbox controller state">
    {@html GAMEPAD_SVG}
  </div>
</section>

<style>
  .gamepad {
    min-height: 100%;
    display: grid;
    overflow: hidden;
    touch-action: none;
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
  .gamepad-shell :global(.gpad-direction-highlight) {
    display: none;
  }

  .gamepad-shell :global(.gpad-highlight) {
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

  .gamepad-shell :global(.is-active.gpad-btn-icon),
  .gamepad-shell :global(.is-active .gpad-btn-icon) {
    stroke: var(--bgDark) !important;
  }

  .gamepad-shell :global(#dpad_up_button_icon.is-active),
  .gamepad-shell :global(#dpad_down_button_icon.is-active),
  .gamepad-shell :global(#dpad_left_button_icon.is-active),
  .gamepad-shell :global(#dpad_right_button_icon.is-active) {
    stroke: var(--bgDark) !important;
  }

  .gamepad-shell :global(.gpad-dpad-bg) {
    fill: var(--bgMedium) !important;
    stroke: var(--borderStrong) !important;
  }

  .gamepad-shell :global(.gpad-dpad-bg.is-active) {
    fill: var(--accent) !important;
    stroke: color-mix(in srgb, var(--accent) 75%, white 25%) !important;
  }

  .gamepad-shell :global(.is-active.gpad-btn-icon > path),
  .gamepad-shell :global(.is-active.gpad-btn-icon > circle),
  .gamepad-shell :global(.is-active .gpad-btn-icon > path),
  .gamepad-shell :global(.is-active .gpad-btn-icon > circle) {
    stroke: inherit !important;
  }

  .gamepad-shell :global(#shoulder_trigger_back_left.is-active .gpad-btn-bg),
  .gamepad-shell :global(#shoulder_trigger_back_right.is-active .gpad-btn-bg) {
    translate: 0 calc(var(--button-value) * 3px);
  }
</style>
