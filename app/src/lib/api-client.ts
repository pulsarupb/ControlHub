import { fetch as tauriFetch } from "@tauri-apps/plugin-http"

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(
      () => reject(new Error(`Rover request timed out after ${timeoutMs} ms`)),
      timeoutMs,
    )
  })
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer))
}

export function apiFetch(
  input: RequestInfo | URL,
  init?: RequestInit,
  timeoutMs = 3000,
): Promise<Response> {
  const request = isTauriRuntime()
    ? tauriFetch(input.toString(), init)
    : fetch(input, init)
  // A hung request must not wedge the control/heartbeat loops forever.
  return withTimeout(request, timeoutMs)
}
