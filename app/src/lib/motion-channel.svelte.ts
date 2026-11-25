import { UI_RATE_MS } from "$lib/constants"
import { roverConnection } from "$lib/rover-connection.svelte"
import type { Devices, StopInfo } from "$lib/hardware-types"
import type { RoverStatus } from "$lib/rover-types"

export type DriveFrame = {
  t: "drive"
  enabled: boolean
  connected: boolean
  simulated: boolean
  updated_at: number | null
  error: string | null
  stop_reason: StopInfo | null
  armed: boolean
  power: number[][]
  faults: number[]
  controllers: Devices["drive"]["data"]["controllers"]
  yaw_control?: Devices["drive"]["data"]["yaw_control"]
  ts: number
}

export type DevicesFrame = {
  t: "devices"
  time: number
  simulate: boolean
  kinova: Devices["kinova"]
  zed: Devices["zed"]
  rtk: Devices["rtk"]
  oak: Devices["oak"]
}

export type StoppedFrame = {
  t: "stopped"
  dev: string[]
  reason: string
}

type AckFrame = { t: "ack"; id?: number; ok: boolean }
type ErrFrame = { t: "err"; id?: number; code: string; msg: string }
type ServerFrame = DriveFrame | DevicesFrame | StoppedFrame | AckFrame | ErrFrame

export class MotionChannelError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message)
    this.name = "MotionChannelError"
  }
}

export type MotionChannelCallbacks = {
  onOpen: () => void
  onClose: (reason: string) => void
  onFallback: () => void
  onStatus: (status: RoverStatus) => void
  onStopped: (devices: string[], reason: string) => void
  onError: (message: string, code: string) => void
  readDriveDemand: () => { armed: boolean; throttle: number; steering: number }
  readArmState: () => {
    active: boolean
    jog: { cartesian: number[] } | { joints: number[] } | null
  }
  clearArmJog: () => void
}

const RECONNECT_MIN_MS = 250
const RECONNECT_MAX_MS = 2000
const COMMAND_TIMEOUT_MS = 1000
const CONTROL_TIMEOUT_MS = 400
const FALLBACK_CONNECT_FAILURES = 4

export function buildWsUrl(baseUrl: string): string {
  const url = new URL(baseUrl)
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:"
  url.pathname = `${url.pathname.replace(/\/$/, "")}/api/ws`
  url.search = ""
  url.hash = ""
  return url.toString()
}

export class MotionChannel {
  private ws: WebSocket | null = null
  private started = false
  private currentBaseUrl = ""
  private nextId = 1
  private reconnectDelay = RECONNECT_MIN_MS
  private consecutiveConnectFailures = 0
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private sendTimer: ReturnType<typeof setInterval> | null = null
  private armCounter = 0
  private controlInFlight = false
  private armInFlight = false
  private readonly pending = new Map<
    number,
    {
      resolve: () => void
      reject: (error: Error) => void
      timer: ReturnType<typeof setTimeout>
    }
  >()

  private driveDevice: Devices["drive"] = {
    enabled: false,
    connected: false,
    simulated: false,
    updated_at: null,
    error: null,
    stop_reason: null,
    data: {
      armed: false,
      power: [
        [0, 0],
        [0, 0],
      ],
      controllers: [
        { encoder_counts: [0, 0], speed_rpm: [0, 0], faults: 0 },
        { encoder_counts: [0, 0], speed_rpm: [0, 0], faults: 0 },
      ],
    },
  }
  private devicesFrame: DevicesFrame | null = null

  constructor(private readonly callbacks: MotionChannelCallbacks) {}

  start(baseUrl = roverConnection.baseUrl): void {
    if (this.started && this.currentBaseUrl === baseUrl && this.ws) return
    this.stop()
    this.started = true
    this.currentBaseUrl = baseUrl
    this.consecutiveConnectFailures = 0
    this.reconnectDelay = RECONNECT_MIN_MS
    this.openSocket()
  }

  stop(): void {
    this.started = false
    this.clearTimers()
    this.rejectAllPending(new Error("Motion channel stopped"))
    if (this.ws) {
      const socket = this.ws
      this.ws = null
      socket.onopen = null
      socket.onmessage = null
      socket.onerror = null
      socket.onclose = null
      try {
        socket.close()
      } catch {
        // ignore close errors during teardown
      }
    }
  }

  isOpen(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN
  }

  enableDrive(): Promise<void> {
    return this.sendRequest({ t: "enable" }, COMMAND_TIMEOUT_MS)
  }

  stopAll(reason = "operator stop"): Promise<void> {
    return this.sendRequest({ t: "stop", r: reason }, COMMAND_TIMEOUT_MS)
  }

  takeControl(): Promise<void> {
    return this.sendRequest({ t: "take" }, COMMAND_TIMEOUT_MS)
  }

  releaseArmJog(): void {
    if (!this.isOpen()) return
    const id = this.nextId++
    this.ws!.send(
      JSON.stringify({ t: "jog", id, c: [0, 0, 0, 0, 0, 0] }),
    )
  }

  private openSocket(): void {
    if (!this.started) return
    let wsUrl: string
    try {
      wsUrl = buildWsUrl(this.currentBaseUrl)
    } catch {
      this.callbacks.onFallback()
      return
    }

    let socket: WebSocket
    try {
      socket = new WebSocket(wsUrl)
    } catch {
      this.handleConnectFailure("WebSocket unavailable")
      return
    }

    let opened = false
    this.ws = socket

    socket.onopen = () => {
      if (this.ws !== socket || !this.started) return
      opened = true
      this.consecutiveConnectFailures = 0
      this.reconnectDelay = RECONNECT_MIN_MS
      this.startSendLoop()
      this.callbacks.onOpen()
    }

    socket.onmessage = (event) => {
      if (this.ws !== socket || typeof event.data !== "string") return
      this.handleMessage(event.data)
    }

    socket.onerror = () => {
      // onclose runs after onerror and drives cleanup + reconnect/fallback.
    }

    socket.onclose = () => {
      if (this.ws !== socket) return
      this.ws = null
      this.stopSendLoop()
      this.rejectAllPending(new Error("Control link closed"))
      if (!this.started) return
      if (!opened) {
        this.handleConnectFailure("WebSocket connection failed")
      } else {
        this.callbacks.onClose("Control link closed")
        this.scheduleReconnect()
      }
    }
  }

  private handleConnectFailure(reason: string): void {
    this.consecutiveConnectFailures += 1
    this.callbacks.onClose(reason)
    if (this.consecutiveConnectFailures >= FALLBACK_CONNECT_FAILURES) {
      this.stop()
      this.callbacks.onFallback()
      return
    }
    this.scheduleReconnect()
  }

  private scheduleReconnect(): void {
    if (!this.started || this.reconnectTimer) return
    const jitter = 0.75 + Math.random() * 0.5
    const delay = Math.round(this.reconnectDelay * jitter)
    this.reconnectDelay = Math.min(RECONNECT_MAX_MS, this.reconnectDelay * 2)
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null
      this.openSocket()
    }, delay)
  }

  private startSendLoop(): void {
    this.stopSendLoop()
    this.sendTimer = setInterval(() => {
      this.tickSend()
    }, UI_RATE_MS)
  }

  private stopSendLoop(): void {
    if (this.sendTimer) {
      clearInterval(this.sendTimer)
      this.sendTimer = null
    }
    this.controlInFlight = false
    this.armInFlight = false
  }

  private clearTimers(): void {
    this.stopSendLoop()
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
  }

  private tickSend(): void {
    if (!this.isOpen()) return

    const drive = this.callbacks.readDriveDemand()
    if (drive.armed && !this.controlInFlight) {
      this.controlInFlight = true
      void this.sendRequest(
        {
          t: "c",
          // Reverse the UI drive axes for this rover without changing backend or motor config.
          th: -drive.throttle,
          st: -drive.steering,
        },
        CONTROL_TIMEOUT_MS,
      )
        .catch((error) => {
          if (error instanceof MotionChannelError) {
            this.callbacks.onError(error.message, error.code)
          }
        })
        .finally(() => {
          this.controlInFlight = false
        })
    }

    this.armCounter = (this.armCounter + 1) % 2
    if (this.armCounter === 0 && !this.armInFlight) {
      const arm = this.callbacks.readArmState()
      if (arm.active) {
        const frame =
          arm.jog && "cartesian" in arm.jog
            ? { t: "jog", c: arm.jog.cartesian }
            : arm.jog && "joints" in arm.jog
              ? { t: "jog", j: arm.jog.joints }
              : { t: "jog" }
        this.armInFlight = true
        void this.sendRequest(frame, COMMAND_TIMEOUT_MS)
          .catch((error) => {
            if (error instanceof MotionChannelError) {
              this.callbacks.clearArmJog()
              this.callbacks.onError(error.message, error.code)
            }
          })
          .finally(() => {
            this.armInFlight = false
          })
      }
    }
  }

  private sendRequest(
    payload: Record<string, unknown>,
    timeoutMs: number,
  ): Promise<void> {
    if (!this.isOpen()) {
      return Promise.reject(new Error("Rover control link offline"))
    }
    const id = this.nextId++
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        reject(new Error(`Control command timed out after ${timeoutMs} ms`))
      }, timeoutMs)
      this.pending.set(id, { resolve, reject, timer })
      try {
        this.ws!.send(JSON.stringify({ ...payload, id }))
      } catch (error) {
        clearTimeout(timer)
        this.pending.delete(id)
        reject(
          error instanceof Error ? error : new Error("Failed to send frame"),
        )
      }
    })
  }

  private rejectAllPending(error: Error): void {
    for (const [id, entry] of this.pending) {
      clearTimeout(entry.timer)
      entry.reject(error)
      this.pending.delete(id)
    }
  }

  private handleMessage(raw: string): void {
    let frame: ServerFrame
    try {
      frame = JSON.parse(raw) as ServerFrame
    } catch {
      return
    }

    if (frame.t === "ack") {
      if (frame.id !== undefined) {
        const entry = this.pending.get(frame.id)
        if (entry) {
          clearTimeout(entry.timer)
          this.pending.delete(frame.id)
          entry.resolve()
        }
      }
      return
    }

    if (frame.t === "err") {
      if (frame.id !== undefined) {
        const entry = this.pending.get(frame.id)
        if (entry) {
          clearTimeout(entry.timer)
          this.pending.delete(frame.id)
          entry.reject(new MotionChannelError(frame.code, frame.msg))
          return
        }
      }
      this.callbacks.onError(frame.msg, frame.code)
      return
    }

    if (frame.t === "stopped") {
      this.callbacks.onStopped(frame.dev, frame.reason)
      return
    }

    if (frame.t === "drive") {
      this.driveDevice = {
        enabled: frame.enabled,
        connected: frame.connected,
        simulated: frame.simulated,
        updated_at: frame.updated_at,
        error: frame.error,
        stop_reason: frame.stop_reason,
        data: {
          armed: frame.armed,
          power: frame.power,
          controllers: frame.controllers,
          yaw_control: frame.yaw_control,
        },
      }
      this.emitStatus(frame.ts)
      return
    }

    if (frame.t === "devices") {
      this.devicesFrame = frame
      this.emitStatus(frame.time)
    }
  }

  private emitStatus(time: number): void {
    if (!this.devicesFrame) return
    this.callbacks.onStatus({
      simulate: this.devicesFrame.simulate,
      time,
      devices: {
        drive: this.driveDevice,
        kinova: this.devicesFrame.kinova,
        zed: this.devicesFrame.zed,
        rtk: this.devicesFrame.rtk,
        oak: this.devicesFrame.oak,
      },
    })
  }
}
