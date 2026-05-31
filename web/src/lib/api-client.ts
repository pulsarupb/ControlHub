import { fetch as tauriFetch } from "@tauri-apps/plugin-http"

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

export function apiFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  if (isTauriRuntime()) {
    return tauriFetch(input.toString(), init)
  }

  return fetch(input, init)
}
