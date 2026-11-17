<script lang="ts">
  import { untrack } from "svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { global } from "$lib"

  // Arm messages stay inline in the Kinova widget (no toast), so they never cover other widgets.
  const drive = $derived(rover.status?.devices.drive)
  const kinova = $derived(rover.status?.devices.kinova)
  const driveStop = $derived(
    drive?.enabled && !drive.data.armed ? (drive.stop_reason?.reason ?? "") : "",
  )
  const armStop = $derived(
    kinova?.enabled && !kinova.data.armed ? (kinova.stop_reason?.reason ?? "") : "",
  )
  // Offline is already shown by the topbar and connection overlay, so only online refusals get a toast.
  const uiError = $derived(
    rover.lastError && rover.connectionState === "online" && !driveStop && !armStop ? rover.lastError : "",
  )

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

  $effect(() => {
    const entries = [
      { key: "drive-stop", text: driveStop ? `Drive stopped: ${driveStop}` : "" },
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
            : []

        notify(entry.key, entry.text, actions)
      }
    })
  })
</script>
