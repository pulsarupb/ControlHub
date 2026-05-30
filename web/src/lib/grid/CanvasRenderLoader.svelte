<script lang="ts">
  import { onMount, setContext, tick } from "svelte"
  import html2canvas from "html2canvas"
  import Layout from "./Layout.svelte"
  import { Manager } from "./widgets.svelte"
  import type { Template } from "$lib/types"
  import { global } from "$lib"
  import Topbar from "$lib/Topbar.svelte"

  let { t }: { t: Template } = $props()

  const cacheKey = $derived(`${t.name}:${JSON.stringify(t)}`)

  const CAPTURE_WIDTH = 1600
  const CAPTURE_HEIGHT = 900
  const TOPBAR_HEIGHT = 72

  let m = $state(new Manager({ name: "Preview", widgets: [], navlets: [] }))
  let targetElement: HTMLDivElement
  let imageDataUrl: string = $state("")

  setContext("manager", () => m)

  function setPreviewGridSize() {
    m.WIDTH = CAPTURE_WIDTH / m.MAX_GRID_WIDTH
    m.HEIGHT = (CAPTURE_HEIGHT - TOPBAR_HEIGHT) / m.MAX_GRID_HEIGHT
  }

  async function waitForImages() {
    const images = [...targetElement.querySelectorAll("img")]

    await Promise.all(
      images.map(async (image) => {
        if (image.complete && image.naturalWidth > 0) return
        await image.decode().catch(() => {})
      })
    )
  }

  async function renderAsImage() {
    if (!targetElement) return

    const rect = targetElement.getBoundingClientRect()
    if (rect.width <= 0 || rect.height <= 0) return

    const renderedCanvas = await html2canvas(targetElement, {
      backgroundColor: null,
      width: CAPTURE_WIDTH,
      height: CAPTURE_HEIGHT,
      windowWidth: CAPTURE_WIDTH,
      windowHeight: CAPTURE_HEIGHT,
    })

    imageDataUrl = renderedCanvas.toDataURL("image/png")
    targetElement.remove()

    global.templatePreviews[cacheKey] = imageDataUrl
  }

  async function waitForStableRender(ticks = 3) {
    for (let i = 0; i < ticks; i++) {
      await tick()
    }
  }

  onMount(async () => {
    m = new Manager(t)
    m.loadPreview(t)
    setPreviewGridSize()
    await waitForStableRender(2)
    setPreviewGridSize()
    await waitForImages()
    await new Promise((resolve) => requestAnimationFrame(resolve))
    await renderAsImage()
  })
</script>

<div
  bind:this={targetElement}
  class="capture-target"
>
  <section class="preview-shell">
    <Topbar />
    <Layout bind:manager={m} enableInteractions={false} preview={true} />
  </section>
</div>

<style>
  .preview-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background: var(--bgDark);
  }
  .capture-target {
    position: fixed;
    left: -10000px;
    top: 0;
    width: 1600px;
    height: 900px;
    z-index: 0;
    pointer-events: none;
    overflow: hidden;
  }
</style>
