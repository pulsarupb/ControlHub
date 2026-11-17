// One shared "are you sure?" for every delete. Deleting runs only after the user accepts.
export const confirmState = $state<{
  open: boolean
  title: string
  message: string
  label: string
  run: (() => void) | null
}>({ open: false, title: "", message: "", label: "Delete", run: null })

export function confirmDelete(title: string, message: string, run: () => void, label = "Delete") {
  confirmState.title = title
  confirmState.message = message
  confirmState.label = label
  confirmState.run = run
  confirmState.open = true
}

export function closeConfirm() {
  confirmState.open = false
  confirmState.run = null
}

export function acceptConfirm() {
  const run = confirmState.run
  closeConfirm()
  run?.()
}
