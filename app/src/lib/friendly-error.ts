// Turns a thrown value into a line a rover operator can read.
// A failed network request is a TypeError in browsers, which almost always means the rover is unreachable.
export function friendlyError(error: unknown, fallback = "Request failed"): string {
  if (error instanceof TypeError) return "Rover unreachable"
  if (error instanceof Error && error.message) return error.message
  if (typeof error === "string" && error) return error
  return fallback
}
