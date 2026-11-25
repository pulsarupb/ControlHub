import { fetch as tauriFetch } from "@tauri-apps/plugin-http"

export type ApiRequestInit = RequestInit & {
  connectTimeout?: number
}

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

export function apiFetch(
  input: RequestInfo | URL,
  init?: ApiRequestInit,
  timeoutMs = 3000,
): Promise<Response> {
  const controller = new AbortController()
  const externalSignal = init?.signal
  const onExternalAbort = () => controller.abort(externalSignal?.reason)

  if (externalSignal) {
    if (externalSignal.aborted) {
      controller.abort(externalSignal.reason)
    } else {
      externalSignal.addEventListener("abort", onExternalAbort, { once: true })
    }
  }

  let timer: ReturnType<typeof setTimeout> | undefined
  // Abort the underlying request on timeout instead of only racing a JS timer;
  // otherwise stalled HTTP calls keep sockets and Tauri reqwest handles alive.
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => {
      const error = new Error(`Rover request timed out after ${timeoutMs} ms`)
      controller.abort(error)
      reject(error)
    }, timeoutMs)
  })

  const { connectTimeout, ...fetchInit } = init ?? {}
  const request = isTauriRuntime()
    ? tauriFetch(input.toString(), {
        ...fetchInit,
        signal: controller.signal,
        ...(connectTimeout !== undefined ? { connectTimeout } : {}),
      })
    : fetch(input, { ...fetchInit, signal: controller.signal })

  // A hung request must not wedge status, camera or config refreshes forever.
  return Promise.race([request, timeout]).finally(() => {
    clearTimeout(timer)
    externalSignal?.removeEventListener("abort", onExternalAbort)
  })
}
