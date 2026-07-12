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

  const buttonActions: Record<string, () => void> = {}

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

    setActive("dpad_up", buttons[xboxButtonMap.DPadUp]?.active ?? false)
    setActive("dpad_down", buttons[xboxButtonMap.DPadDown]?.active ?? false)
    setActive("dpad_left", buttons[xboxButtonMap.DPadLeft]?.active ?? false)
    setActive("dpad_right", buttons[xboxButtonMap.DPadRight]?.active ?? false)

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

    const container = svg.querySelector("#dpad_up")?.parentElement
    if (!container) return

    container.innerHTML = ""

    const tapTargets = svg.querySelector("#dpad_tap_targets")
    if (tapTargets) {
      container.appendChild(tapTargets)
    }

    const buttons = [
      { id: "dpad_up", cx: 77, cy: 79, icon: "M77 74l5 8H72Z" },
      { id: "dpad_down", cx: 77, cy: 117, icon: "M77 122l-5-8h10Z" },
      { id: "dpad_left", cx: 58, cy: 98, icon: "M53 98l8-5v10Z" },
      { id: "dpad_right", cx: 97, cy: 98, icon: "M102 98l-8-5v10Z" },
    ]

    for (const { id, cx, cy, icon } of buttons) {
      const group = document.createElementNS("http://www.w3.org/2000/svg", "g")
      group.id = id
      group.style.pointerEvents = "none"

      const shadow = document.createElementNS("http://www.w3.org/2000/svg", "circle")
      shadow.setAttribute("cx", String(cx + 0.86))
      shadow.setAttribute("cy", String(cy + 0.85))
      shadow.setAttribute("r", "11.806")
      shadow.classList.add("gpad-shadow")
      group.appendChild(shadow)

      const bg = document.createElementNS("http://www.w3.org/2000/svg", "circle")
      bg.setAttribute("cx", String(cx))
      bg.setAttribute("cy", String(cy))
      bg.setAttribute("r", "11.8")
      bg.classList.add("gpad-btn-bg")
      group.appendChild(bg)

      const border = document.createElementNS("http://www.w3.org/2000/svg", "path")
      border.setAttribute("d", `M${cx} ${cy - 11.8}c6.512 0 11.8 5.287 11.8 11.8 0 6.512-5.288 11.8-11.8 11.8s-11.8-5.288-11.8-11.8 5.287-11.8 11.8-11.8m0 1.72c-5.563 0-10.08 4.516-10.08 10.08 0 5.563 4.517 10.079 10.08 10.079s10.079-4.516 10.079-10.079-4.516-10.08-10.079-10.08`)
      group.appendChild(border)

      const iconGroup = document.createElementNS("http://www.w3.org/2000/svg", "g")
      iconGroup.classList.add("gpad-btn-icon")

      const iconPath = document.createElementNS("http://www.w3.org/2000/svg", "path")
      iconPath.setAttribute("d", icon)
      iconPath.style.fill = "none"
      iconPath.style.stroke = "#000"
      iconPath.style.strokeWidth = "1.72px"
      iconPath.style.strokeLinejoin = "round"
      iconGroup.appendChild(iconPath)
      group.appendChild(iconGroup)

      container.appendChild(group)
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
