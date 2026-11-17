<script lang="ts">
  import { type ExtendedWidgetGroup } from "../widgets.svelte"
  import Portal from "svelte-portal"
  import WidgetDrag from "./WidgetDrag.svelte"
  import WidgetResize from "./WidgetResize.svelte"
  import { getComponent } from "$lib/widgets/registry"
  import { getContext, tick } from "svelte"
  import WidgetAdd from "./WidgetAdd.svelte"
  import IconTrashRegular from "phosphor-icons-svelte/IconTrashRegular.svelte"
  import IconXRegular from "phosphor-icons-svelte/IconXRegular.svelte"
  import { confirmDelete } from "$lib/ui/confirm.svelte"
  import type { Manager } from "../widgets.svelte"
  const manager = getContext("manager") as Manager

  let {
    widget = $bindable(),
    isPossible = $bindable(),
  }: { widget: ExtendedWidgetGroup; isPossible: boolean } = $props()

  let movingIndex = $state(0)

  // While a tab is dragged near either end of the strip, the strip scrolls so the tab can reach the first or last slot.
  const EDGE_ZONE = 56
  let tabStrip: HTMLElement | null = null
  let edgeDirection = 0
  let edgeStrength = 0
  let edgeFrame = 0

  function autoScrollStep() {
    if (!tabStrip || edgeDirection === 0) {
      edgeFrame = 0
      return
    }
    tabStrip.scrollLeft += edgeDirection * (3 + 13 * edgeStrength)
    edgeFrame = requestAnimationFrame(autoScrollStep)
  }

  function updateEdgeScroll(x: number) {
    if (!tabStrip) return
    const rect = tabStrip.getBoundingClientRect()
    if (x < rect.left + EDGE_ZONE) {
      edgeDirection = -1
      edgeStrength = Math.min(1, (rect.left + EDGE_ZONE - x) / EDGE_ZONE)
    } else if (x > rect.right - EDGE_ZONE) {
      edgeDirection = 1
      edgeStrength = Math.min(1, (x - (rect.right - EDGE_ZONE)) / EDGE_ZONE)
    } else {
      edgeDirection = 0
    }
    if (edgeDirection !== 0 && !edgeFrame) edgeFrame = requestAnimationFrame(autoScrollStep)
  }

  function stopEdgeScroll() {
    edgeDirection = 0
    if (edgeFrame) cancelAnimationFrame(edgeFrame)
    edgeFrame = 0
    tabStrip = null
  }

  // Removes one tab. The last tab is emptied instead, so the group keeps its place on the grid.
  function removeTab(index: number) {
    if (widget.widgets.length === 1) {
      widget.widgets[0] = { isMoving: false, widgetID: "" }
      return
    }
    widget.widgets.splice(index, 1)
    if (widget.selected >= widget.widgets.length) widget.selected = widget.widgets.length - 1
  }

  // Adds a tab holding the widget the user picked, and selects it.
  async function addWidget(widgetID: string) {
    widget.widgets.push({ isMoving: false, widgetID })
    widget.selected = widget.widgets.length - 1
    await tick()
    const tab = document.querySelector<HTMLElement>(`[data-widget="${widget.id}"][data-index="${widget.selected}"]`)
    if (tab) scrollTabWithinGroup(tab)
  }

  let tabX = $state(0)
  let tabY = $state(0)

  let xOffset = $state(0)
  let yOffset = $state(0)

  let pointerId: number | null = null

  function scrollTabWithinGroup(tab: HTMLElement) {
    const tabs = tab.closest<HTMLElement>(".tabs")
    if (!tabs) return

    const tabRect = tab.getBoundingClientRect()
    const tabsRect = tabs.getBoundingClientRect()
    const left = tabs.scrollLeft + tabRect.left - tabsRect.left - (tabs.clientWidth - tabRect.width) / 2
    tabs.scrollTo({ left: Math.max(0, left), behavior: "smooth" })
  }

  // Touch: a quick sideways swipe scrolls the tab strip, a tap selects the tab, and holding still
  // for a moment starts the move. Moving sideways or down before the hold cancels it, so a swipe
  // never drags a widget out by accident.
  const LONG_PRESS_MS = 350
  const SLOP_PX = 8
  type TouchPress = {
    id: number
    x: number
    y: number
    index: number
    element: HTMLElement
    tabs: HTMLElement | null
    scrollLeft: number
    timer: ReturnType<typeof setTimeout> | null
    scrolling: boolean
    event: PointerEvent
  }
  let touchPress: TouchPress | null = null

  function beginTouchPress(e: PointerEvent, index: number) {
    if (!e.isPrimary) return
    const element = e.currentTarget as HTMLElement
    const tabs = element.closest<HTMLElement>(".tabs")
    const press: TouchPress = {
      id: e.pointerId,
      x: e.clientX,
      y: e.clientY,
      index,
      element,
      tabs,
      scrollLeft: tabs?.scrollLeft ?? 0,
      timer: null,
      scrolling: false,
      event: e,
    }
    press.timer = setTimeout(() => {
      if (touchPress !== press) return
      endTouchPress()
      movingIndex = index
      startMove(press.event)
    }, LONG_PRESS_MS)
    touchPress = press
    window.addEventListener("pointermove", onTouchPressMove)
    window.addEventListener("pointerup", onTouchPressEnd)
    window.addEventListener("pointercancel", onTouchPressEnd)
  }

  function endTouchPress() {
    const press = touchPress
    if (press?.timer) clearTimeout(press.timer)
    touchPress = null
    window.removeEventListener("pointermove", onTouchPressMove)
    window.removeEventListener("pointerup", onTouchPressEnd)
    window.removeEventListener("pointercancel", onTouchPressEnd)
    return press
  }

  function onTouchPressMove(e: PointerEvent) {
    const press = touchPress
    if (!press || e.pointerId !== press.id) return
    const dx = e.clientX - press.x
    const dy = e.clientY - press.y
    if (!press.scrolling && Math.max(Math.abs(dx), Math.abs(dy)) > SLOP_PX) {
      if (press.tabs && Math.abs(dx) > Math.abs(dy)) {
        press.scrolling = true
        if (press.timer) clearTimeout(press.timer)
        press.timer = null
      } else {
        endTouchPress()
        return
      }
    }
    if (press.scrolling && press.tabs) press.tabs.scrollLeft = press.scrollLeft - dx
  }

  // A tap selects its tab from the click, not from pointerup: selecting on pointerup reveals the
  // tab's close button under the finger, and the click then lands on it.
  let suppressTabClick = false

  function onTouchPressEnd(e: PointerEvent) {
    const press = touchPress
    if (!press || e.pointerId !== press.id) return
    endTouchPress()
    if (press.scrolling) suppressTabClick = true
  }

  function selectTab(index: number, element: HTMLElement) {
    if (suppressTabClick) {
      suppressTabClick = false
      return
    }
    if (isPossible || !manager.enableInteractions) return
    widget.selected = index
    scrollTabWithinGroup(element)
  }

  function startMove(e: PointerEvent) {
    e.preventDefault()
    if (!manager.enableInteractions) return
    if (isPossible) return
    console.log("Started move of", movingIndex)

    manager.tabName = getWidgetName(widget.widgets[movingIndex].widgetID)
    pointerId = e.pointerId

    window.addEventListener("pointermove", onMove)
    document.body.classList.add("no-select")

    const tab = document.querySelector(
      `[data-widget="${widget.id}"][data-index="${movingIndex}"]`
    ) as HTMLElement

    xOffset = 0
    yOffset = 0

    const rect = tab?.getBoundingClientRect()

    if (rect) {
      xOffset = e.clientX - rect.left
      yOffset = e.clientY - rect.top
    }

    tabStrip = tab?.closest<HTMLElement>(".tabs") ?? null
    tabX = e.clientX
    tabY = e.clientY
    window.addEventListener("pointerup", stopMove)
    window.addEventListener("pointercancel", stopMove)
  }

  function getWidgetName(widgetID: string) {
    return getComponent(widgetID, "widget")?.name ?? "Empty"
  }

  function onMove(e: PointerEvent) {
    if (!manager.enableInteractions || e.pointerId !== pointerId) return
    tabX = e.clientX
    tabY = e.clientY
    updateEdgeScroll(e.clientX)
    if (
      Math.abs(e.clientX - xOffset) >= 1 &&
      Math.abs(e.clientY - yOffset) >= 1
    ) {
      widget.widgets[movingIndex].isMoving = true
    }

    const elements = document.elementsFromPoint(e.clientX, e.clientY)
    const el = elements.filter(
      (el): el is HTMLElement =>
        el instanceof HTMLElement &&
        el.hasAttribute("data-widget") &&
        el.hasAttribute("data-index")
    )[0]

    if (el == undefined) {
      manager.tabIndex = 0
      manager.tabWidgetID = ""
      return
    }

    let dataWidget = el.getAttribute("data-widget") || ""
    let dataIndex = parseInt(el.getAttribute("data-index") || "")
    if (
      dataWidget != widget.id ||
      (dataWidget == widget.id &&
        movingIndex != dataIndex &&
        movingIndex != dataIndex + 1)
    ) {
      manager.tabIndex = dataIndex
      manager.tabWidgetID = dataWidget
    }
  }
  function stopMove(e: PointerEvent) {
    if (!manager.enableInteractions || e.pointerId !== pointerId) return
    stopEdgeScroll()
    console.log("Stopped move of", movingIndex)
    widget.widgets[movingIndex].isMoving = false

    manager.tabIndex = 0
    manager.tabWidgetID = ""
    manager.tabName = ""

    const elements = document.elementsFromPoint(e.clientX, e.clientY)
    const el = elements.filter(
      (el): el is HTMLElement =>
        el instanceof HTMLElement &&
        el.hasAttribute("data-widget") &&
        el.hasAttribute("data-index")
    )[0]

    if (el == undefined) {
      window.removeEventListener("pointermove", onMove)
      window.removeEventListener("pointerup", stopMove)
      window.removeEventListener("pointercancel", stopMove)
      pointerId = null
      document.body.classList.remove("no-select")
      return
    }

    scrollTabWithinGroup(el)

    let dataWidget = el.getAttribute("data-widget")
    let dataIndex = parseInt(el.getAttribute("data-index") || "")

    if (
      dataWidget != widget.id ||
      (dataWidget == widget.id &&
        movingIndex != dataIndex &&
        movingIndex != dataIndex + 1)
    ) {
      console.log(el, dataWidget, dataIndex)

      const replaceWidget = manager.widgets.find((it) => it.id == dataWidget)

      if (replaceWidget != undefined && dataIndex != undefined) {
        const movingPanel = { ...widget.widgets[movingIndex] }

        widget.widgets.splice(movingIndex, 1)

        replaceWidget.widgets.splice(
          0,
          replaceWidget.widgets.length,
          ...replaceWidget.widgets.slice(0, dataIndex + 1),
          movingPanel,
          ...replaceWidget.widgets.slice(dataIndex + 1)
        )

        widget.selected -= 1
        if (widget.selected < 0) {
          widget.selected = 0
        }
        if (widget.selected > widget.widgets.length - 1) {
          widget.selected = widget.widgets.length - 1
        }
      }
    } else {
      if (dataIndex == movingIndex) {
        widget.selected = movingIndex
      }
    }

    window.removeEventListener("pointermove", onMove)
    window.removeEventListener("pointerup", stopMove)
    window.removeEventListener("pointercancel", stopMove)
    pointerId = null
    document.body.classList.remove("no-select")

    manager.save()
  }
</script>

{#if widget.widgets?.[movingIndex]?.isMoving}
  <Portal>
    <div class="drag-chip" style="left:{tabX - xOffset}px;top:{tabY - yOffset}px;">{manager.tabName}</div>
  </Portal>
{/if}

<nav>
  <WidgetDrag bind:widget bind:isPossible />
  <span class="divider" aria-hidden="true"></span>
  <button
    class="ghost danger"
    aria-label="Delete group"
    title="Delete group"
    onclick={() => confirmDelete("Delete group?", `This deletes the group and its ${widget.widgets.length} widget${widget.widgets.length === 1 ? "" : "s"}.`, () => manager.removeWidget(widget.id))}
  >
    <IconTrashRegular />
  </button>
  <div class="tabs">
    {#each widget.widgets as w, index}
      <div class="tab-wrap" class:selected={index === widget.selected && !w.isMoving}>
        <button
          class="tab"
          data-widget={widget.id}
          data-index={index}
          class:emptyMoving={w.isMoving}
          class:selected={index === widget.selected && !w.isMoving}
          onclick={(e: MouseEvent) => {
            if (w.isMoving || (widget.widgets.length === 1 && w.widgetID === "")) return
            selectTab(index, e.currentTarget as HTMLElement)
          }}
          onpointerdown={(e: PointerEvent) => {
            if (widget.widgets.length === 1 && w.widgetID === "") return
            if (e.pointerType !== "mouse") {
              beginTouchPress(e, index)
              return
            }
            movingIndex = index

            startMove(e)

            const element = e.currentTarget as HTMLElement

            scrollTabWithinGroup(element)
          }}>{getWidgetName(w.widgetID)}</button
        >
        {#if index === widget.selected && !w.isMoving}
          <button class="tab-close" aria-label="Remove this widget" title="Remove this widget" onclick={() => confirmDelete("Remove widget?", "This removes the widget from its group.", () => removeTab(index))}>
            <IconXRegular />
          </button>
        {/if}
      </div>
      {#if manager.tabWidgetID === widget.id && manager.tabIndex === index}
        <span class="drop-marker" aria-hidden="true"></span>
      {/if}
    {/each}
    {#if widget.widgets.length > 0}
      <WidgetAdd onpick={addWidget} />
    {/if}
    {#if widget.widgets.length === 0}
      <div class="tab-wrap selected">
        <button class="tab" data-widget={widget.id} data-index={-1}>
          {#if manager.tabWidgetID === widget.id && manager.tabIndex === -1}
            {manager.tabName}
          {:else}
            Empty
          {/if}
        </button>
      </div>
    {/if}
  </div>
  <div class="actions" data-workspace-preview-ignore>
    <div class="top-resize">
      <WidgetResize bind:widget bind:isPossible />
    </div>
  </div>
</nav>

<style>
  /* One visual language for the whole bar: ghost icon buttons, text tabs in pills, one height. */
  nav {
    --ctl: 1.6rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: var(--inset) var(--inset) calc(var(--inset) / 2) var(--inset);
    width: 100%;
    box-sizing: border-box;
  }
  .divider {
    flex: none;
    width: 1px;
    height: 1rem;
    background: var(--border);
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    overflow-y: auto;
    user-select: none;
    min-width: 0;
    scrollbar-width: none;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    margin-left: auto;
  }

  /* Ghost button: no border, no fill until hovered. Every header icon uses it. */
  .ghost {
    flex: none;
    display: grid;
    place-items: center;
    width: var(--ctl);
    height: var(--ctl);
    padding: 0;
    border: 0;
    border-radius: 0.4rem;
    background: transparent;
    color: var(--textSubtle);
    font-size: 0.95rem;
    cursor: pointer;
  }
  .ghost:hover { background: var(--bgLight); color: var(--text); }
  .ghost.danger:hover { color: var(--danger); }
  .ghost:focus-visible,
  .tab-close:focus-visible,
  .tab:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }

  /* A tab is a pill: name on the left, and on the selected tab its own remove button. */
  .tab-wrap {
    flex: none;
    display: flex;
    align-items: center;
    height: var(--ctl);
    border: 1px solid transparent;
    border-radius: 0.4rem;
  }
  .tab-wrap.selected {
    background: var(--surfaceRaised);
    border-color: var(--borderStrong);
  }
  .tab {
    all: unset;
    box-sizing: border-box;
    height: 100%;
    display: inline-flex;
    align-items: center;
    padding: 0 0.55rem;
    font-size: 0.85rem;
    color: var(--textMuted);
    text-wrap: nowrap;
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
  }
  .tab:hover { color: var(--text); }
  .tab.selected { color: var(--text); }
  .tab-close {
    all: unset;
    box-sizing: border-box;
    display: grid;
    place-items: center;
    width: 1.25rem;
    height: 1.25rem;
    margin-right: 0.2rem;
    border-radius: 0.3rem;
    color: var(--textSubtle);
    font-size: 0.7rem;
    cursor: pointer;
  }
  .tab-close:hover { color: var(--text); background: var(--bgLight); }
  .emptyMoving{
    opacity: 0.25;
  }
  /* Insertion point while dragging a tab: one bar, no duplicate label. */
  .drop-marker {
    flex: none;
    width: 2px;
    height: 1.25rem;
    border-radius: 1px;
    background: var(--primary);
  }
  /* Narrow widgets (same threshold as Widget.svelte): hide the top-bar resize handle, shorten tab labels.
     Declared after the base rules so these overrides win. */
  @container (max-width: 180px) {
    /* Resize and the menu move to the corner stack in Widget.svelte. */
    .actions {
      display: none;
    }
    .tab {
      max-width: 6em;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }
  /* The dragged tab, drawn as a pill that follows the pointer. Fixed, so it can leave the group. */
  .drag-chip {
    position: fixed;
    z-index: 1000;
    pointer-events: none;
    display: inline-flex;
    align-items: center;
    height: 1.6rem;
    padding: 0 0.55rem;
    border: 1px solid var(--borderStrong);
    border-radius: 0.4rem;
    background: var(--surfaceRaised);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.45);
    color: var(--text);
    font-size: 0.85rem;
    white-space: nowrap;
  }
</style>
