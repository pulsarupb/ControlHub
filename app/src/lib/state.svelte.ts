import { templates } from "$lib/widgets/registry"
import type { Notification, NotificationAction, Template } from "$lib/types"

let nextNotificationId = 0

class NotificationsManager {
  data: Notification[] = $state([])

  private push(text: string, actions: NotificationAction[]) {
    const id = `notification-${++nextNotificationId}`
    this.data.push({ id, text, timestamp: Date.now(), actions, read: false })
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

  markAllRead() {
    for (const notification of this.data) notification.read = true
  }

  clear() {
    this.data = []
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
