import { templates } from "$lib/widgets/registry"
import type { Notification, NotificationAction, Template } from "$lib/types"

let nextNotificationId = 0
const MAX_HISTORY = 100

class NotificationsManager {
  data: Notification[] = $state([])

  private push(text: string, actions: NotificationAction[]) {
    const id = `notification-${++nextNotificationId}`
    this.data.push({ id, text, timestamp: Date.now(), actions, read: false })

    // Keep history bounded without dropping alerts that are still actionable.
    if (this.data.length > MAX_HISTORY) {
      const index = this.data.findIndex(
        (notification) => notification.actions.length === 0 || notification.executedAction,
      )
      if (index !== -1) this.data.splice(index, 1)
    }

    return id
  }

  add(text: string) {
    return this.push(text, [])
  }

  addAction(text: string, actions: NotificationAction[]) {
    return this.push(text, actions)
  }

  remove(id: string) {
    const index = this.data.findIndex((notification) => notification.id === id)
    if (index !== -1) this.data.splice(index, 1)
  }

  /** Hide the toast but keep the notification in the list as history. */
  dismiss(id: string) {
    const notification = this.data.find((it) => it.id === id)
    if (notification) notification.executedAction = "Dismiss"
  }

  /** Mark an action as handled so the toast hides while the entry stays in history. */
  markActionExecuted(id: string, action: string) {
    const notification = this.data.find((it) => it.id === id)
    if (notification) notification.executedAction = action
  }

  markAllRead() {
    for (const notification of this.data) notification.read = true
  }

  /** Clear history while keeping alerts that still need an action. */
  clear() {
    this.data = this.data.filter(
      (notification) => notification.actions.length > 0 && !notification.executedAction,
    )
  }
}

export class GlobalState {
  notificationsManager = new NotificationsManager()
  templatePreviews: Record<string, string> = $state({})
  isPrepared = $state(true)
  isConnected = $state(true)

  notifications = $derived(this.notificationsManager.data)
  unreadNotifications = $derived(
    this.notificationsManager.data.filter((notification) => !notification.read).length,
  )
  allTemplates: Template[] = $state(templates)
}
