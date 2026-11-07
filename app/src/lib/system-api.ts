import { apiFetch } from "$lib/api-client"
import { roverConnection } from "$lib/rover-connection.svelte"

export type PowerAction = "shutdown" | "reboot"

export class PowerRequestError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message)
    this.name = "PowerRequestError"
  }
}

export async function requestPower(action: PowerAction): Promise<void> {
  const response = await apiFetch(
    roverConnection.apiUrl(`/api/system/${action}`),
    {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ confirm: true }),
    },
    10_000,
  )
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as {
      error?: string
    } | null
    throw new PowerRequestError(
      response.status,
      body?.error ?? `Power request failed with ${response.status}`,
    )
  }
}

export async function roverHealthOk(): Promise<boolean> {
  try {
    const response = await apiFetch(
      roverConnection.apiUrl("/api/health"),
      { headers: { accept: "application/json" } },
      2000,
    )
    return response.ok
  } catch {
    return false
  }
}
