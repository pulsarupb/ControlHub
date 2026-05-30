export class RoverData {
  startedAt = Date.now()
  tick = $state(0)
  gamepadConnected = $state(false)
  gamepadId = $state("No gamepad connected")
  gamepadAxes = $state([0, 0, 0, 0])
  gamepadButtons = $state<boolean[]>(Array.from({ length: 17 }, () => false))

  battery = $derived(84 + Math.sin(this.tick / 18) * 5)
  voltage = $derived(22.4 + Math.sin(this.tick / 21) * 0.8)
  signal = $derived(72 + Math.sin(this.tick / 10) * 16)
  cpuTemp = $derived(48 + Math.sin(this.tick / 14) * 7)
  heading = $derived((this.tick * 3) % 360)
  speed = $derived(Math.max(0, 0.7 + Math.sin(this.tick / 8) * 0.45))
  distance = $derived(24.8 + Math.sin(this.tick / 25) * 4)
  mode = $derived(this.tick % 90 > 45 ? "Autonomy" : "Standby")

  telemetry = $derived([
    `Battery: ${this.battery.toFixed(1)}%`,
    `Voltage: ${this.voltage.toFixed(2)} V`,
    `Signal: ${this.signal.toFixed(0)}%`,
    `Heading: ${this.heading.toFixed(0)} deg`,
    `Speed: ${this.speed.toFixed(2)} m/s`,
    `Obstacle: ${this.distance.toFixed(1)} m`,
  ])

  samples = $state<{ t: number; battery: number; signal: number; speed: number }[]>([])

  start() {
    if (typeof window === "undefined") return
    this.pushSample()
    this.pollGamepad()
    window.setInterval(() => {
      this.tick += 1
      this.pushSample()
    }, 1000)
    const frame = () => {
      this.pollGamepad()
      window.requestAnimationFrame(frame)
    }
    window.requestAnimationFrame(frame)
  }

  private pollGamepad() {
    if (typeof navigator === "undefined" || !navigator.getGamepads) return
    const gamepad = Array.from(navigator.getGamepads()).find(Boolean)
    if (!gamepad) {
      this.gamepadConnected = false
      this.gamepadId = "No gamepad connected"
      this.gamepadAxes = [0, 0, 0, 0]
      this.gamepadButtons = Array.from({ length: 17 }, () => false)
      return
    }

    this.gamepadConnected = true
    this.gamepadId = gamepad.id
    this.gamepadAxes = Array.from({ length: 4 }, (_, index) => gamepad.axes[index] ?? 0)
    this.gamepadButtons = Array.from({ length: 17 }, (_, index) => gamepad.buttons[index]?.pressed ?? false)
  }

  private pushSample() {
    const t = Math.round((Date.now() - this.startedAt) / 1000)
    this.samples = [
      ...this.samples.slice(-59),
      {
        t,
        battery: this.battery,
        signal: this.signal,
        speed: this.speed * 50,
      },
    ]
  }
}

export const rover = new RoverData()
