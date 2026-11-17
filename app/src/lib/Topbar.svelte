<script module lang="ts">
  declare const __APP_VERSION__: string
</script>

<script lang="ts">
  import Panels from "$lib/Panels.svelte"
  import Button from "$lib/ui/Button.svelte"
  import Overlay from "$lib/ui/Overlay.svelte"
  import { global } from "$lib"
  import Navlets from "./navlets/Navlets.svelte"

  import IconBellRegular from "phosphor-icons-svelte/IconBellRegular.svelte"
  import IconCopyRegular from "phosphor-icons-svelte/IconCopyRegular.svelte"
  import IconGearSixRegular from "phosphor-icons-svelte/IconGearSixRegular.svelte"
  import IconPlusRegular from "phosphor-icons-svelte/IconPlusRegular.svelte"
  import IconRobotRegular from "phosphor-icons-svelte/IconRobotRegular.svelte"
  import IconSlidersHorizontalRegular from "phosphor-icons-svelte/IconSlidersHorizontalRegular.svelte"
  import IconTrashRegular from "phosphor-icons-svelte/IconTrashRegular.svelte"
  import IconMonitorArrowUpRegular from "phosphor-icons-svelte/IconMonitorArrowUpRegular.svelte"
  import { getContext } from "svelte"
  import type { Manager } from "./grid/widgets.svelte"
  import TemplateMiniature from "./grid/TemplateMiniature.svelte"
  import TemplateLive from "./grid/TemplateLive.svelte"
  import PreviewCard from "./ui/PreviewCard.svelte"
  import AllNotifications from "./AllNotifications.svelte"
  import RoverConnectionOverlay from "$lib/rover/RoverConnectionOverlay.svelte"
  import SettingsOverlay from "$lib/SettingsOverlay.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"

  import { check } from "@tauri-apps/plugin-updater"
  import { relaunch } from "@tauri-apps/plugin-process"
  import { isTauri } from "@tauri-apps/api/core"
  import { androidUpdater, type AndroidUpdateCheck } from "$lib/android-updater"
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow"
  import { onMount, onDestroy } from "svelte"
  import ConfirmDialog from "$lib/ui/ConfirmDialog.svelte"
  import { confirmDelete } from "$lib/ui/confirm.svelte"



  let { preview = false }: { preview?: boolean } = $props()

  let versionDismissed = $state(false)
  let updateAvailable = $state(false)
  let updateError = $state(false)
  let downloading = $state(false)
  let downloadProgress = $state(0)
  let polling = $state<ReturnType<typeof setInterval> | null>(null)
  let progressPolling = $state<ReturnType<typeof setInterval> | null>(null)
  let pendingUpdate: Awaited<ReturnType<typeof check>> = $state(null)
  let pendingAndroidUpdate = $state<AndroidUpdateCheck | null>(null)
  let androidApp = $state(false)
  let updateMessage = $state("")

  async function pollForUpdate() {
    try {
      if (androidApp) {
        const update = await androidUpdater.check()
        pendingAndroidUpdate = update.available ? update : null
        updateAvailable = update.available
        updateError = false
        updateMessage = ""
        return
      }

      const update = await check()
      if (update?.available) {
        pendingUpdate = update
        updateAvailable = true
        updateError = false
      }
    } catch {
      // silent — don't nag on transient failures
    }
  }

  async function downloadAndInstallAndroid() {
    const update = pendingAndroidUpdate
    if (!update) return

    if (update.requiresReinstall) {
      const confirmed = window.confirm(
        "This installation was signed with a different key. To enable in-app updates, back up any important app data, uninstall Panels ERC Edition, then install the signed APK from the release page. Uninstalling may erase app data. Open the release page now?",
      )
      if (!confirmed) return
      updateMessage = "A one-time reinstall is required before in-app updates can work."
      updateError = true
      try {
        await androidUpdater.openReleasePage()
      } catch (error) {
        console.error("Unable to open Android release page:", error)
      }
      return
    }

    downloading = true
    updateError = false
    updateMessage = ""
    downloadProgress = 0
    const refreshProgress = async () => {
      try {
        const progress = await androidUpdater.progress()
        if (progress.totalBytes > 0) {
          downloadProgress = progress.downloadedBytes / progress.totalBytes
        }
      } catch {
        // Progress is best-effort; the native download reports any actual failure.
      }
    }

    try {
      progressPolling = setInterval(refreshProgress, 300)
      await androidUpdater.download()
      await refreshProgress()
      downloadProgress = 1

      const result = await androidUpdater.install()
      if (result.permissionRequired) {
        updateMessage = "Allow Panels ERC Edition to install apps in Android settings, then tap Install again."
        updateError = true
        return
      }
      updateMessage = "Confirm the update in Android's installer."
    } catch (error) {
      console.error("Android update failed:", error)
      updateMessage = error instanceof Error ? error.message : String(error)
      updateError = true
    } finally {
      if (progressPolling) clearInterval(progressPolling)
      progressPolling = null
      downloading = false
    }
  }

  async function downloadAndInstall() {
    if (androidApp) {
      await downloadAndInstallAndroid()
      return
    }

    downloading = true
    let downloaded = 0
    let contentLength = 0
    try {
      if (!pendingUpdate?.available) {
        downloading = false
        updateAvailable = false
        return
      }
      await pendingUpdate.downloadAndInstall((e) => {
        if (e.event === "Started") {
          downloaded = 0
          contentLength = e.data.contentLength ?? 0
          downloadProgress = 0
        }
        if (e.event === "Progress") {
          downloaded += e.data.chunkLength
          downloadProgress = contentLength > 0 ? downloaded / contentLength : 0
        }
        if (e.event === "Finished") downloadProgress = 1
      })
      await relaunch()
    } catch (e) {
      console.error("Update failed:", e)
      updateError = true
      downloading = false
    }
  }

  onMount(() => {
    if (preview) return

    androidApp = isTauri() && /Android/i.test(navigator.userAgent)
    pollForUpdate()
    polling = setInterval(pollForUpdate, 600_000)
  })

  onDestroy(() => {
    if (polling) clearInterval(polling)
    if (progressPolling) clearInterval(progressPolling)
  })
  let showSettings = $state(false)

  let showVersionWarning = $derived(
    !versionDismissed &&
      roverConnection.serverVersion !== null &&
      roverConnection.serverVersion !== __APP_VERSION__,
  )

  const contextManager = getContext("manager") as Manager | (() => Manager)
  let manager = $state(
    typeof contextManager === "function" ? contextManager() : contextManager,
  )
  let jsonPreset = $state("")

  const kasmDesktopUrl = $derived.by(() => {
    try {
      const url = new URL(roverConnection.baseUrl)
      return `http://${url.hostname}:6901/`
    } catch {
      return "http://10.42.0.1:6901/"
    }
  })

  function openKasmDesktop() {
    try {
      const w = new WebviewWindow("kasm-desktop", {
        url: kasmDesktopUrl,
        title: "Rover Desktop",
        width: 1280,
        height: 820,
        resizable: true,
      })
      w.once("tauri://error", (e) => console.error("Failed to open Kasm desktop", e))
    } catch {
      window.open(kasmDesktopUrl, "_blank")
    }
  }
</script>

<nav>
  <div class="drag-area">
    <a href="/" aria-label="Dashboard home">
      <Panels />
    </a>

    <Navlets bind:manager />
  </div>

  <div class="right-section">
    <div class="right-actions">
      {#if showVersionWarning}
        <button
          class="version-warning"
          onclick={() => (versionDismissed = true)}
        >
          ⚠ v{__APP_VERSION__} - v{roverConnection.serverVersion}
        </button>
      {/if}

      {#if downloading}
        <button class="update-btn downloading" disabled>
          {Math.round(downloadProgress * 100)}%
        </button>
      {:else if updateError}
        <button class="update-btn error" title={updateMessage} onclick={downloadAndInstall}>
          {pendingAndroidUpdate?.requiresReinstall ? "Open reinstall instructions" : "Retry update"}
        </button>
      {:else if updateAvailable}
        <button
          class="update-btn available"
          title={pendingAndroidUpdate?.notes ?? "Install the available update"}
          onclick={downloadAndInstall}
        >
          {pendingAndroidUpdate?.requiresReinstall ? "Reinstall required" : `Update ${pendingAndroidUpdate?.version ?? "available"}`}
        </button>
      {/if}

      <Overlay
        triggerStyle="display: flex;justify-content: center;align-items: center;"
      >
        {#snippet trigger()}
          <span
            class="topbar-icon"
            class:online={roverConnection.state === "online"}
            aria-label="Rover connection"
          >
            <IconRobotRegular />
          </span>
        {/snippet}
        {#snippet overlay()}
          <RoverConnectionOverlay />
        {/snippet}
      </Overlay>

      <span
        class="topbar-icon"
        class:online={roverConnection.state === "online"}
        aria-label="Open rover desktop"
        role="button"
        tabindex="0"
        onclick={openKasmDesktop}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault()
            openKasmDesktop()
          }
        }}
      >
        <IconMonitorArrowUpRegular />
      </span>

      <Overlay
        triggerStyle="display: flex;justify-content: center;align-items: center;"
        onStateChange={(isOpen) => {
          if (isOpen) global.notificationsManager.markAllRead()
        }}
      >
        {#snippet trigger()}
          <span class="topbar-icon" aria-label="Notifications">
            <IconBellRegular />
            {#if global.unreadNotifications > 0}
              <span class="notification-badge" aria-hidden="true">
                {global.unreadNotifications > 9 ? "9+" : global.unreadNotifications}
              </span>
            {/if}
          </span>
        {/snippet}
        {#snippet overlay()}
          <div class="bell-menu">
            <div class="bell-header">
              <h1>Notifications</h1>
              {#if global.notifications.length > 0}
                <Button transparent onclick={() => global.notificationsManager.clear()}>
                  Clear
                </Button>
              {/if}
            </div>
            {#if global.notifications.length > 0}
              <AllNotifications />
            {:else}
              <p>No notifications</p>
            {/if}
          </div>
        {/snippet}
      </Overlay>

      <Overlay triggerStyle="display: flex;justify-content: center;align-items: center;">
        {#snippet trigger()}
          <span class="topbar-icon" aria-label="Presets">
            <IconSlidersHorizontalRegular />
          </span>
        {/snippet}
        {#snippet overlay({ close }: { close: () => void })}
          <div class="presets-overlay">
            <header class="presets-head">
              <h1>Presets</h1>
              <button class="text-btn" onclick={() => { manager.newPreset(); close() }}>
                <IconPlusRegular /> New empty
              </button>
            </header>

            <ul class="preset-list">
              {#each manager.presets.data as preset, index}
                <li class:selected={manager.presets.selected === index}>
                  <button class="pick" aria-label="Open {preset.name}" aria-pressed={manager.presets.selected === index} onclick={() => manager.change(index)}>
                    <span class="dot"></span>
                  </button>
                  <input
                    class="name"
                    aria-label="Preset name"
                    bind:value={manager.presets.data[index].name}
                    oninput={() => manager.save()}
                  />
                  <button
                    class="icon"
                    aria-label="Copy preset JSON"
                    title="Copy JSON"
                    onclick={() => {
                      manager.save()
                      const temp = manager.unprocessTemplate(manager.presets.data[index])
                      navigator.clipboard
                        .writeText(JSON.stringify(temp))
                        .then(() => global.notificationsManager.add("Text copied to clipboard"))
                        .catch(() => global.notificationsManager.add("Failed to copy to clipboard"))
                    }}
                  >
                    <IconCopyRegular />
                  </button>
                  <button
                    class="icon danger"
                    aria-label="Delete preset"
                    title={manager.presets.data.length === 1 ? "Keep at least one preset" : "Delete preset"}
                    disabled={manager.presets.data.length === 1}
                    onclick={() => confirmDelete("Delete preset?", `"${manager.presets.data[index].name}" and everything on it will be removed.`, () => manager.deletePreset(index))}
                  >
                    <IconTrashRegular />
                  </button>
                </li>
              {/each}
            </ul>

            <section>
              <h2>Add a template</h2>
              <div class="tpl-grid">
                {#each global.allTemplates as t}
                  <PreviewCard lazy name={t.name} detail="{t.widgets.length} groups · {t.navlets.length} navlets" onclick={() => { manager.addTemplate(t); close() }}>
                    <TemplateLive {t} />
                  </PreviewCard>
                {/each}
              </div>
            </section>

            <section class="import">
              <h2>Import JSON</h2>
              <textarea bind:value={jsonPreset} placeholder="Paste a preset JSON"></textarea>
              <button
                class="text-btn primary"
                disabled={jsonPreset.trim() === ""}
                onclick={() => {
                  try {
                    manager.addTemplate(JSON.parse(jsonPreset))
                    jsonPreset = ""
                    close()
                  } catch {
                    global.notificationsManager.add("Invalid JSON preset")
                  }
                }}
              >
                Import
              </button>
            </section>
          </div>
        {/snippet}
      </Overlay>

      <span
        class="topbar-icon"
        aria-label="Settings"
        role="button"
        tabindex="0"
        onclick={() => (showSettings = true)}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault()
            showSettings = true
          }
        }}
      >
        <IconGearSixRegular />
      </span>
    </div>
  </div>
</nav>

<SettingsOverlay show={showSettings} onclose={() => (showSettings = false)} />
<ConfirmDialog />

<style>
  h1 {
    margin: 0.5rem;
    font-size: 1.5rem;
  }
  a {
    color: inherit;
    text-decoration: none;
  }
  .presets-overlay {
    width: min(760px, calc(100vw - 32px));
    max-height: min(84vh, 780px);
    overflow-y: auto;
    padding: var(--inset);
    display: grid;
    gap: 1rem;
    font-size: 0.85rem;
  }
  .presets-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .presets-overlay h1 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }
  .presets-overlay h2 {
    margin: 0 0 0.4rem;
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--textSubtle);
  }
  .text-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    height: 1.7rem;
    padding: 0 0.6rem;
    border: 1px solid var(--border);
    border-radius: 0.4rem;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .text-btn:hover { background: var(--bgLight); }
  .text-btn.primary { border-color: var(--borderStrong); background: var(--bgLight); }
  .preset-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.3rem;
  }
  .preset-list li {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.2rem 0.3rem 0.2rem 0.2rem;
    border: 1px solid var(--border);
    border-radius: 0.45rem;
    background: var(--bgDark);
  }
  .preset-list li.selected { border-color: var(--primary); }
  .pick {
    all: unset;
    box-sizing: border-box;
    flex: none;
    width: 1.6rem;
    height: 1.6rem;
    display: grid;
    place-items: center;
    border-radius: 0.35rem;
    cursor: pointer;
  }
  .pick:hover { background: var(--bgLight); }
  .pick .dot {
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    border: 1.5px solid var(--borderStrong);
  }
  li.selected .pick .dot { background: var(--primary); border-color: var(--primary); }
  .name {
    flex: 1;
    min-width: 0;
    padding: 0.25rem 0.4rem;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 0.35rem;
    color: var(--text);
    font: inherit;
  }
  .name:hover { border-color: var(--border); }
  .name:focus { outline: none; border-color: var(--primary); }
  .icon {
    all: unset;
    box-sizing: border-box;
    flex: none;
    width: 1.6rem;
    height: 1.6rem;
    display: grid;
    place-items: center;
    border-radius: 0.35rem;
    color: var(--textSubtle);
    cursor: pointer;
  }
  .icon:hover { background: var(--bgLight); color: var(--text); }
  .icon.danger:hover { color: var(--danger); }
  .icon:disabled { background: none; color: var(--textSubtle); }
  .tpl-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(440px, 1fr));
    gap: 0.75rem;
  }
  .import textarea {
    width: 100%;
    box-sizing: border-box;
    min-height: 4.5rem;
    resize: vertical;
    padding: 0.45rem 0.55rem;
    background: var(--bgDark);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 0.4rem;
    font: inherit;
    font-family: ui-monospace, monospace;
    font-size: 0.8rem;
  }
  .import .text-btn { margin-top: 0.4rem; }
  .bell-menu {
    padding: calc(var(--padding) / 2);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--padding) / 2);
  }
  .bell-menu {
    min-width: 300px;
    max-width: 420px;
  }
  .bell-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--padding);
    width: 100%;
  }
  .bell-header h1 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }
  .bell-header :global(button) {
    padding: 0.25em 0.6em;
    font-size: 0.8rem;
  }
  .version-warning {
    border: 1px solid color-mix(in srgb, var(--danger) 65%, transparent);
    border-radius: 0.4rem;
    padding: 0.38rem 0.75rem;
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--textBad);
    font-size: 0.8rem;
    cursor: pointer;
    font-family: inherit;
    white-space: nowrap;
  }
  .version-warning:hover {
    background: color-mix(in srgb, var(--danger) 18%, transparent);
  }
  .update-btn {
    border-radius: 0.4rem;
    padding: 0.38rem 0.75rem;
    font-size: 0.8rem;
    cursor: pointer;
    font-family: inherit;
    white-space: nowrap;
    border: 1px solid color-mix(in srgb, var(--success) 60%, transparent);
    background: color-mix(in srgb, var(--success) 12%, transparent);
    color: var(--textOk);
  }
  .update-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--success) 18%, transparent);
  }
  .update-btn:disabled {
    cursor: not-allowed;
    opacity: 0.65;
  }
  .update-btn.error {
    border-color: color-mix(in srgb, var(--danger) 60%, transparent);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--textBad);
  }
  .update-btn.error:hover {
    background: color-mix(in srgb, var(--danger) 18%, transparent);
  }
  .topbar-icon {
    position: relative;
    width: 28px;
    height: 36px;
    display: grid;
    place-items: center;
    color: var(--textMuted);
    font-size: 1.3rem;
  }
  .notification-badge {
    position: absolute;
    top: 1px;
    right: -2px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--danger);
    color: var(--text);
    font-size: 0.6rem;
    font-weight: 700;
    line-height: 1;
    display: grid;
    place-items: center;
    pointer-events: none;
  }
  .topbar-icon:hover {
    color: var(--text);
  }
  .topbar-icon.online {
    color: var(--textOk);
  }
  nav {
    background-color: var(--bgMedium);
    display: flex;
    align-items: stretch;
    border: 1px solid var(--border);
    border-radius: 0.55rem;
    margin: var(--gutter);
    margin-bottom: 0;
    gap: var(--padding);
    max-width: 100%;
    overflow: hidden;
    min-height: 48px;
  }
  .drag-area {
    display: flex;
    align-items: center;
    gap: var(--padding);
    padding: 0 var(--padding);
    flex: 1;
    min-width: 0;
  }
  .right-section {
    display: flex;
    align-items: stretch;
    margin-left: auto;
    pointer-events: auto;
  }
  /* Wrapping is driven by content, not a breakpoint: the icon row moves to a second line only
     when the logo, status and navlets no longer fit on one. */
  nav {
    flex-wrap: wrap;
    row-gap: 0;
  }
  /* Shares the row with the icons: it takes the space left over and scrolls its navlets inside it. */
  .drag-area {
    flex: 1 1 0;
    min-width: 0;
    flex-wrap: wrap;
    row-gap: 0.4rem;
  }
  .right-section {
    flex: 0 0 auto;
    margin-left: auto;
  }
  .right-section:only-child,
  .right-section:not(:first-child) {
    margin-left: auto;
  }
  /* Phone: the logo and the icons share the first row; the navlets wrap to a second row beneath them. */
  @media (max-width: 520px) {
    .drag-area {
      padding: 0.25rem var(--padding);
      flex: 0 1 auto;
      gap: 0.5rem;
    }
    .right-section {
      margin-left: auto;
    }
    .right-actions {
      gap: 0.1rem;
      padding: 0 0.25rem;
    }
  }
  .right-actions {
    display: flex;
    align-items: center;
    gap: calc(var(--padding) / 2);
    padding: 0 var(--padding);
  }
</style>
