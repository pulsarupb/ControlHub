/** Capture the mounted dashboard once, then cut it into six workspace thumbnails.
 * Only the cloned document is changed: no scrolling or duplicate widget instances.
 */
export async function captureWorkspacePreviews(canvas: HTMLElement) {
  const { default: html2canvas } = await import("html2canvas")
  if (!canvas.isConnected) return null

  const width = canvas.clientWidth
  const height = canvas.clientHeight
  if (!width || !height) return null
  const background = getComputedStyle(canvas).getPropertyValue("--bgDark").trim() || "#101214"
  const snapshot = await html2canvas(canvas, {
    width,
    height,
    scale: Math.min(1, 480 / (width / 3)),
    backgroundColor: background,
    logging: false,
    allowTaint: false,
    imageTimeout: 1500,
    ignoreElements: (element) => element.hasAttribute("data-workspace-preview-ignore"),
    onclone: (document, clonedCanvas) => {
      // Reveal the full 3 × 2 canvas, including workspaces outside the viewport.
      for (let parent = clonedCanvas.parentElement; parent; parent = parent.parentElement) {
        parent.style.overflow = "visible"
        parent.style.scrollBehavior = "auto"
        parent.scrollLeft = 0
        parent.scrollTop = 0
      }
      clonedCanvas.style.background = background

      // External media can taint screenshots; WebGL/video/iframes cannot be
      // reliably snapshotted. Preserve their geometry with a quiet placeholder.
      clonedCanvas.querySelectorAll("canvas, video, iframe, img").forEach((element) => {
        if (element.tagName === "IMG") {
          const image = element as HTMLImageElement
          const source = image.currentSrc || image.src
          if (!source || new URL(source, document.baseURI).origin === location.origin || /^(data:|blob:)/.test(source)) return
        }
        const placeholder = document.createElement("div")
        const style = document.defaultView!.getComputedStyle(element)
        placeholder.style.cssText = `width:${style.width};height:${style.height};box-sizing:border-box;display:grid;place-items:center;background:${background};color:#9ba4ae;font:14px sans-serif;overflow:hidden;`
        placeholder.textContent = element.tagName === "IFRAME" ? "Embedded content" : "Media preview"
        element.replaceWith(placeholder)
      })
    },
  })

  try {
    const previews: string[] = []
    const thumbnail = document.createElement("canvas")
    thumbnail.width = Math.max(1, Math.round(snapshot.width / 3))
    thumbnail.height = Math.max(1, Math.round(snapshot.height / 2))
    try {
      const context = thumbnail.getContext("2d")
      if (!context) return null
      for (let row = 0; row < 2; row++) {
        for (let column = 0; column < 3; column++) {
          context.clearRect(0, 0, thumbnail.width, thumbnail.height)
          context.drawImage(snapshot,
            column * snapshot.width / 3, row * snapshot.height / 2,
            snapshot.width / 3, snapshot.height / 2,
            0, 0, thumbnail.width, thumbnail.height,
          )
          previews.push(thumbnail.toDataURL("image/png"))
        }
      }
      return { images: previews, aspectRatio: `${width / 3} / ${height / 2}` }
    } finally {
      thumbnail.width = thumbnail.height = 0
    }
  } finally {
    snapshot.width = snapshot.height = 0
  }
}
