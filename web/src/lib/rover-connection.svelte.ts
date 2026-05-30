import type { ConnectionState } from "$lib/moteus-types"

const DEFAULT_ROVER_URL = "http://127.0.0.1:8080"
const STORAGE_KEY = "roverUrl"

type HealthResponse = {
  service?: string
  ok?: boolean
}

type ValidationResult = {
  ok: boolean
  message: string
  latencyMs?: number
}

function storedUrl(): string {
  if (typeof localStorage === "undefined") return DEFAULT_ROVER_URL

  return localStorage.getItem(STORAGE_KEY) || DEFAULT_ROVER_URL
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
  state = $state<ConnectionState>("connecting")
  message = $state("Choose a rover and run healthcheck")
  latencyMs = $state<number | null>(null)
  lastCheckedAt = $state<number | null>(null)
  isChecking = $state(false)

  apiUrl(path: string): string {
    return `${this.baseUrl}${path.startsWith("/") ? path : `/${path}`}`
  }

  setState(state: ConnectionState, message?: string) {
    this.state = state
    if (message) this.message = message
  }

  useLocal() {
    this.draftUrl = DEFAULT_ROVER_URL
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

      if (typeof localStorage !== "undefined") {
        localStorage.setItem(STORAGE_KEY, normalizedUrl)
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
