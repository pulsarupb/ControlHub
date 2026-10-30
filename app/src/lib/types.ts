import type { Component } from "svelte"

export type ComponentKind = "widget" | "navlet"

export type DashboardComponent = {
  id: string
  name: string
  type: ComponentKind
  component: Component
  minW?: number
  minH?: number
  maxW?: number
  maxH?: number
}

export type TemplateWidget = {
  widgetID: string
}

export type TemplateWidgetGroup = {
  x: number
  y: number
  w: number
  h: number
  widgets: TemplateWidget[]
}

export type TemplateNavlet = {
  navletID: string
}

export type Template = {
  name: string
  widgets: TemplateWidgetGroup[]
  navlets: TemplateNavlet[]
}

export type NotificationAction = {
  text: string
  task: () => void
}

export type Notification = {
  text: string
  timestamp: number
  actions: NotificationAction[]
  executedAction?: string
}
