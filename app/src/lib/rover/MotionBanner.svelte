<script lang="ts">
  import { untrack } from "svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { hardware, armAction } from "$lib/hardware.svelte"
  import { global } from "$lib"

  const drive = $derived(rover.status?.devices.drive)
  const kinova = $derived(rover.status?.devices.kinova)
  const driveStop = $derived(
    drive?.enabled && !drive.data.armed ? (drive.stop_reason?.reason ?? "") : "",
  )
  const armStop = $derived(
    kinova?.enabled && !kinova.data.armed ? (kinova.stop_reason?.reason ?? "") : "",
  )
  const armError = $derived(
    hardware.error && kinova?.enabled && !kinova.data.armed ? hardware.error : "",
  )
  const uiError = $derived(rover.lastError && !driveStop && !armStop ? rover.lastError : "")

  type ActiveNotification = { id: string; text: string }
  const active = new Map<string, ActiveNotification>()

  function notify(
    key: string,
    text: string,
    actions: { text: string; task: () => void }[],
  ) {
    const existing = active.get(key)
    if (existing?.text === text) return
    if (existing) global.notificationsManager.dismiss(existing.id)

    let id = ""
    id = global.notificationsManager.addAction(text, [
      ...actions,
      { text: "Dismiss", task: () => global.notificationsManager.dismiss(id) },
    ])
    active.set(key, { id, text })
  }

  function dismiss(key: string) {
    const existing = active.get(key)
    if (!existing) return
    // Keep the notification in the bell list as history; only hide its toast.
    global.notificationsManager.dismiss(existing.id)
    active.delete(key)
  }

  async function enableArm() {
    await armAction({ action: "enable" })
    await rover.refreshStatus()
  }

  $effect(() => {
    const entries = [
      { key: "drive-stop", text: driveStop ? `Drive stopped: ${driveStop}` : "" },
      { key: "arm-stop", text: armStop ? `Arm stopped: ${armStop}` : "" },
      { key: "arm-error", text: armError ? `Arm error: ${armError}` : "" },
      { key: "rover-error", text: uiError ? `Rover error: ${uiError}` : "" },
    ]

    untrack(() => {
      for (const entry of entries) {
        if (!entry.text) {
          dismiss(entry.key)
          continue
        }

        const actions =
          entry.key === "drive-stop"
            ? [{ text: "Enable Drive", task: () => void rover.resetRover() }]
            : entry.key === "arm-stop"
              ? [{ text: "Enable arm", task: () => void enableArm() }]
              : []

        notify(entry.key, entry.text, actions)
      }
    })
  })
</script>
