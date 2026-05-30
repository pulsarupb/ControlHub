import type { ConnectionState } from "$lib/moteus-types"

const ROVER_LOCAL_URL = "http://10.42.0.1:8080"
const DEFAULT_REMOTE_URL = "http://127.0.0.1:8080"
const STORAGE_KEY = "roverUrl"
const MODE_KEY = "roverMode"

type HealthResponse = {
  service?: string
  ok?: boolean
}

type ValidationResult = {
  ok: boolean
  message: string
  latencyMs?: number
}

function storedMode(): "local" | "remote" {
  if (typeof localStorage === "undefined") return "local"
  const m = localStorage.getItem(MODE_KEY)
  if (m === "local" || m === "remote") return m
  return "local"
}

function storedUrl(): string {
  if (typeof localStorage === "undefined") return ROVER_LOCAL_URL
  if (storedMode() === "local") return ROVER_LOCAL_URL
  return localStorage.getItem(STORAGE_KEY) || DEFAULT_REMOTE_URL
}

function normalizeUrl(value: string): string {
  const trimmed = value.trim()
  if (!trimmed) return ""

  const withProtocol = /^https?:\/\//i.test(trimmed) ? trimmed : `http://${trimmed}`
  const url = new URL(withProtocol)
  url.pathname = url.pathname.replace(/\/$/, "")
  url.search = ""
  url.hash = ""
  return url.toString().replace(/\/$/, "")
}

class RoverConnection {
  baseUrl = $state(storedUrl())
  draftUrl = $state(storedUrl())
  connectionMode = $state<"local" | "remote">(storedMode())
  state = $state<ConnectionState>("connecting")
  message = $state("Choose a rover and run healthcheck")
  latencyMs = $state<number | null>(null)
  lastCheckedAt = $state<number | null>(null)
  isChecking = $state(false)

  roverSsid = $state("Pulsar-Rover")

  apiUrl(path: string): string {
    return `${this.baseUrl}${path.startsWith("/") ? path : `/${path}`}`
  }

  setState(state: ConnectionState, message?: string) {
    this.state = state
    if (message) this.message = message
  }

  useLocal() {
    this.connectionMode = "local"
    this.draftUrl = ROVER_LOCAL_URL
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(MODE_KEY, "local")
    }
  }

  useRemote() {
    this.connectionMode = "remote"
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(MODE_KEY, "remote")
    }
  }

  async connect(url = this.draftUrl): Promise<ValidationResult> {
    this.isChecking = true
    this.state = "connecting"
    this.message = "Checking rover health..."

    try {
      const normalizedUrl = normalizeUrl(url)
      const startedAt = performance.now()
      const response = await fetch(`${normalizedUrl}/api/health`, {
        headers: { accept: "application/json" },
      })
      const latencyMs = Math.round(performance.now() - startedAt)

      if (!response.ok) {
        throw new Error(`Healthcheck returned ${response.status}`)
      }

      const data = (await response.json()) as HealthResponse
      if (data.service !== "pulsar-rover" || data.ok !== true) {
        throw new Error("Healthcheck did not identify a Pulsar rover")
      }

      this.baseUrl = normalizedUrl
      this.draftUrl = normalizedUrl
      this.state = "online"
      this.message = "Rover healthcheck passed"
      this.latencyMs = latencyMs
      this.lastCheckedAt = Date.now()

      if (typeof localStorage !== "undefined" && this.connectionMode === "remote") {
        localStorage.setItem(STORAGE_KEY, normalizedUrl)
        localStorage.setItem(MODE_KEY, "remote")
      }

      return { ok: true, message: this.message, latencyMs }
    } catch (error) {
      this.state = "offline"
      this.latencyMs = null
      this.lastCheckedAt = Date.now()
      this.message = error instanceof Error ? error.message : "Rover healthcheck failed"
      return { ok: false, message: this.message }
    } finally {
      this.isChecking = false
    }
  }
}

export const roverConnection = new RoverConnection()
