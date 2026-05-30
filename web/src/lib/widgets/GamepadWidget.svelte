<script lang="ts">
  import { rover } from "$lib/data/rover.svelte"

  type Stick = {
    x: number
    y: number
    value: boolean
  }

  type GamepadView = {
    l1: boolean
    l2: number
    r1: boolean
    r2: number
    leftStick: Stick
    rightStick: Stick
    cross: boolean
    circle: boolean
    square: boolean
    triangle: boolean
    dpad_up: boolean
    dpad_left: boolean
    dpad_right: boolean
    dpad_down: boolean
    touchpad: boolean
    options: boolean
    share: boolean
    ps: boolean
  }

  const emptyGamepad = (): GamepadView => ({
    l1: false,
    l2: 0,
    r1: false,
    r2: 0,
    leftStick: { x: 0, y: 0, value: false },
    rightStick: { x: 0, y: 0, value: false },
    cross: false,
    circle: false,
    square: false,
    triangle: false,
    dpad_up: false,
    dpad_left: false,
    dpad_right: false,
    dpad_down: false,
    touchpad: false,
    options: false,
    share: false,
    ps: false,
  })

  const gamepad = $derived.by<GamepadView>(() => {
    const buttons = rover.gamepadButtons
    const axes = rover.gamepadAxes
    if (!rover.gamepadConnected) return emptyGamepad()

    return {
      l1: buttons[4] ?? false,
      l2: buttons[6] ? 1 : 0,
      r1: buttons[5] ?? false,
      r2: buttons[7] ? 1 : 0,
      leftStick: {
        x: axes[0] ?? 0,
        y: axes[1] ?? 0,
        value: buttons[10] ?? false,
      },
      rightStick: {
        x: axes[2] ?? 0,
        y: axes[3] ?? 0,
        value: buttons[11] ?? false,
      },
      cross: buttons[0] ?? false,
      circle: buttons[1] ?? false,
      square: buttons[2] ?? false,
      triangle: buttons[3] ?? false,
      dpad_up: buttons[12] ?? false,
      dpad_down: buttons[13] ?? false,
      dpad_left: buttons[14] ?? false,
      dpad_right: buttons[15] ?? false,
      touchpad: buttons[17] ?? false,
      options: buttons[9] ?? false,
      share: buttons[8] ?? false,
      ps: buttons[16] ?? false,
    }
  })

  function boolValue(value: boolean) {
    return value ? 1 : 0
  }

  function analogValue(value: number) {
    return Math.max(0, Math.min(1, value))
  }

  function stickStyle(stick: Stick) {
    return `--offsetX:${stick.x};--offsetY:${stick.y};--value:${stick.value ? 1 : Math.max(Math.abs(stick.x), Math.abs(stick.y))};`
  }

</script>

<section class="gamepad">
  <div class="drawing reversed">
    <img src="/gamepad-front.png" alt="Gamepad front" draggable="false" />
    <span class="button" aria-label="L1" style="--top:35%;--left:30.5%;--value:{boolValue(gamepad.l1)}"></span>
    <span class="button" aria-label="L2" style="--top:46.4%;--left:30.9%;--value:{analogValue(gamepad.l2)}"></span>
    <span class="button" aria-label="R1" style="--top:35%;--left:69.5%;--value:{boolValue(gamepad.r1)}"></span>
    <span class="button" aria-label="R2" style="--top:46.4%;--left:69.1%;--value:{analogValue(gamepad.r2)}"></span>
  </div>

  <div class="drawing normal">
    <img src="/gamepad-top.png" alt="Gamepad top" draggable="false" />
    <span class="button" style="--top:25.8%;--left:30%;--value:{boolValue(gamepad.dpad_up)}"></span>
    <span class="button" style="--top:36.9%;--left:30%;--value:{boolValue(gamepad.dpad_down)}"></span>
    <span class="button" style="--top:30.9%;--left:26.4%;--value:{boolValue(gamepad.dpad_left)}"></span>
    <span class="button" style="--top:30.9%;--left:33.4%;--value:{boolValue(gamepad.dpad_right)}"></span>

    <span class="button" style="--top:22.7%;--left:70%;--value:{boolValue(gamepad.triangle)}"></span>
    <span class="button" style="--top:39.8%;--left:70%;--value:{boolValue(gamepad.cross)}"></span>
    <span class="button" style="--top:31%;--left:74.8%;--value:{boolValue(gamepad.circle)}"></span>
    <span class="button" style="--top:31%;--left:65.3%;--value:{boolValue(gamepad.square)}"></span>

    <span class="button touchpad" style="--top:23.9%;--left:50%;--value:{boolValue(gamepad.touchpad)}"></span>
    <span class="button" style="--top:47.9%;--left:50%;--value:{boolValue(gamepad.ps)}"></span>
    <span class="button" style="--top:19.7%;--left:37%;--value:{boolValue(gamepad.share)}"></span>
    <span class="button" style="--top:19.7%;--left:63%;--value:{boolValue(gamepad.options)}"></span>

    <span class="button" style="--top:60%;--left:40%;--value:{boolValue(gamepad.leftStick.value)}"></span>
    <span class="button" style="--top:60%;--left:60%;--value:{boolValue(gamepad.rightStick.value)}"></span>
    <span class="stick" style="--top:45%;--left:40%;{stickStyle(gamepad.leftStick)}"></span>
    <span class="stick" style="--top:45%;--left:60%;{stickStyle(gamepad.rightStick)}"></span>
  </div>

</section>

<style>
  .gamepad {
    flex-grow: 1;
    min-width: 300px;
    min-height: 100%;
    position: relative;
    overflow: hidden;
    --primary: var(--accent);
  }
  .drawing {
    position: relative;
    aspect-ratio: 1920 / 1080;
    height: auto;
  }
  .reversed {
    margin-bottom: -20%;
    margin-top: -15%;
  }
  img {
    width: 100%;
    height: 100%;
    position: absolute;
    object-fit: contain;
    user-select: none;
  }
  .button,
  .stick {
    --value: 0;
    position: absolute;
    top: var(--top);
    left: var(--left);
    background-color: var(--primary);
    transform: translateX(-50%);
    width: 4%;
    aspect-ratio: 1 / 1;
    border-radius: 69vw;
    opacity: calc(0.2 + var(--value) * 0.75);
    box-shadow: 0 0 calc(var(--value) * 28px) rgba(56, 189, 248, 0.75);
  }
  .touchpad {
    width: 12%;
    border-radius: 1rem;
    aspect-ratio: 3 / 1.5;
  }
  .stick {
    width: 6.5%;
    top: calc(var(--top) + var(--offsetY) * 3%);
    left: calc(var(--left) + var(--offsetX) * 3%);
  }
</style>
