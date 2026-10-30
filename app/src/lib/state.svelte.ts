import { templates } from "$lib/widgets/registry"
import type { Notification, NotificationAction, Template } from "$lib/types"

class NotificationsManager {
  data: Notification[] = $state([])

  add(text: string) {
    this.data.push({ text, timestamp: Date.now(), actions: [] })
  }

  addAction(text: string, actions: NotificationAction[]) {
    this.data.push({ text, timestamp: Date.now(), actions })
  }
}

export class GlobalState {
  notificationsManager = new NotificationsManager()
  templatePreviews: Record<string, string> = $state({})
  isPrepared = $state(true)
  isConnected = $state(true)

  notifications = $derived(this.notificationsManager.data)
  allTemplates: Template[] = $state(templates)
}
