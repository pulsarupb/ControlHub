<script lang="ts">
  import { onMount } from "svelte"
  import { SwitchInput } from "$lib/switch-input/vendor"

  const input = new SwitchInput()
  const selector = [
    "button:not(:disabled)",
    "a[href]",
    "[role='button']",
    "input:not(:disabled)",
    "select:not(:disabled)",
    "textarea:not(:disabled)",
    "[tabindex]:not([tabindex='-1'])",
  ].join(",")

  function candidates() {
    return [...document.querySelectorAll<HTMLElement>(selector)].filter((element) => {
      if (element.closest("[inert], [aria-hidden='true']")) return false
      const rect = element.getBoundingClientRect()
      if (element.tabIndex < 0 && element.hasAttribute("role")) element.tabIndex = -1
      return rect.width > 0 && rect.height > 0 && getComputedStyle(element).visibility !== "hidden"
    })
  }

  function isEditing(element: Element | null) {
    return element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement ||
      element instanceof HTMLSelectElement || (element instanceof HTMLElement && element.isContentEditable)
  }

  function moveFocus(direction: "UP" | "DOWN" | "LEFT" | "RIGHT") {
    const items = candidates()
    if (!items.length) return
    const current = document.activeElement as HTMLElement | null
    if (!current || !items.includes(current)) {
      items[0].focus()
      return
    }
    const from = current.getBoundingClientRect()
    const cx = from.left + from.width / 2
    const cy = from.top + from.height / 2
    let best: HTMLElement | undefined
    let bestScore = Infinity
    for (const item of items) {
      if (item === current) continue
      const rect = item.getBoundingClientRect()
      const dx = rect.left + rect.width / 2 - cx
      const dy = rect.top + rect.height / 2 - cy
      const primary = direction === "LEFT" ? -dx : direction === "RIGHT" ? dx : direction === "UP" ? -dy : dy
      if (primary <= 0) continue
      const secondary = direction === "LEFT" || direction === "RIGHT" ? Math.abs(dy) : Math.abs(dx)
      const score = primary + secondary * 2.5
      if (score < bestScore) { best = item; bestScore = score }
    }
    best?.focus()
  }

  onMount(() => {
    void input.start({
      buttons: { mode: "push" },
      axes: { mode: "off" },
      touch: { mode: "off" },
      imu: { mode: "off" },
    })
    const unsubscribe = input.on("buttons", (event) => {
      if (event.type !== "button" || !event.pressed || event.repeat) return
      const target = document.activeElement
      if (isEditing(target)) return
      if (["UP", "DOWN", "LEFT", "RIGHT"].includes(event.name)) {
        moveFocus(event.name as "UP" | "DOWN" | "LEFT" | "RIGHT")
      } else if (event.name === "A") {
        (target as HTMLElement | null)?.click?.()
      } else if (event.name === "B") {
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }))
      }
    })
    return () => { unsubscribe(); void input.stop() }
  })
</script>
