<script lang="ts">
  import Panels from "$lib/Panels.svelte"
  import Button from "$lib/ui/Button.svelte"
  import Overlay from "$lib/ui/Overlay.svelte"
  import TextInput from "$lib/ui/TextInput.svelte"
  import { global } from "$lib"
  import Navlets from "./navlets/Navlets.svelte"

  import IconBellRegular from "phosphor-icons-svelte/IconBellRegular.svelte"
  import IconCopyRegular from "phosphor-icons-svelte/IconCopyRegular.svelte"
  import IconPlusRegular from "phosphor-icons-svelte/IconPlusRegular.svelte"
  import IconRobotRegular from "phosphor-icons-svelte/IconRobotRegular.svelte"
  import IconSlidersHorizontalRegular from "phosphor-icons-svelte/IconSlidersHorizontalRegular.svelte"
  import IconTrashRegular from "phosphor-icons-svelte/IconTrashRegular.svelte"
  import IconDotsThreeVerticalRegular from "phosphor-icons-svelte/IconDotsThreeVerticalRegular.svelte"
  import { getContext } from "svelte"
  import type { Manager } from "./grid/widgets.svelte"
  import TemplatesChoose from "./grid/TemplatesChoose.svelte"
  import AllNotifications from "./AllNotifications.svelte"
  import RoverConnectionOverlay from "$lib/moteus/RoverConnectionOverlay.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"

  import { check } from "@tauri-apps/plugin-updater"
  import { relaunch } from "@tauri-apps/plugin-process"
  import { onMount, onDestroy } from "svelte"

  declare const __APP_VERSION__: string

  let versionDismissed = $state(false)
  let updateAvailable = $state(false)
  let updateError = $state(false)
  let downloading = $state(false)
  let downloadProgress = $state(0)
  let polling = $state<ReturnType<typeof setInterval> | null>(null)
  let pendingUpdate: Awaited<ReturnType<typeof check>> = $state(null)

  async function pollForUpdate() {
    try {
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

  async function downloadAndInstall() {
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
    pollForUpdate()
    polling = setInterval(pollForUpdate, 600_000)
  })

  onDestroy(() => {
    if (polling) clearInterval(polling)
  })
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
      {:else if updateAvailable}
        <button class="update-btn available" onclick={downloadAndInstall}>
          Update available
        </button>
      {:else if updateError}
        <button class="update-btn error" onclick={downloadAndInstall}>
          Update failed - Retry
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

      <Overlay
        triggerStyle="display: flex;justify-content: center;align-items: center;"
      >
        {#snippet trigger()}
          <span class="topbar-icon" aria-label="Notifications">
            <IconBellRegular />
          </span>
        {/snippet}
        {#snippet overlay()}
          <div class="bell-menu">
            <h1>Notifications</h1>
            {#if global.notifications.length > 0}
              <AllNotifications />
            {:else}
              <p>No notifications</p>
            {/if}
          </div>
        {/snippet}
      </Overlay>

      <Overlay
        triggerStyle="display: flex;justify-content: center;align-items: center;"
      >
        {#snippet trigger()}
          <span class="topbar-icon" aria-label="Presets">
            <IconSlidersHorizontalRegular />
          </span>
        {/snippet}
        {#snippet overlay({ close }: { close: () => void })}
          <div class="presets-overlay">
            <h1>Presets</h1>
            {#each manager.presets.data as preset, index}
              <div class="preset">
                <Button
                  selected={manager.presets.selected === index}
                  onclick={() => {
                    manager.change(index)
                  }}
                >
                  {preset.name}
                </Button>
                <Overlay>
                  {#snippet trigger()}
                    <IconDotsThreeVerticalRegular />
                  {/snippet}
                  {#snippet overlay({ close }: { close: () => void })}
                    <div class="menu">
                      <TextInput
                        bind:value={manager.presets.data[index].name}
                        oninput={() => {
                          manager.save()
                        }}
                      />
                      <Button
                        transparent={true}
                        disabled={manager.presets.data.length === 1}
                        onclick={() => {
                          manager.deletePreset(index)
                          close()
                        }}
                      >
                        <IconTrashRegular />
                      </Button>
                      <Button
                        transparent={true}
                        onclick={() => {
                          manager.save()
                          const temp = manager.unprocessTemplate(
                            manager.presets.data[index],
                          )
                          close()
                          global.notificationsManager.addAction(
                            JSON.stringify(temp),
                            [
                              {
                                text: "Copy",
                                task: () => {
                                  navigator.clipboard
                                    .writeText(JSON.stringify(temp))
                                    .then(() => {
                                      global.notificationsManager.add(
                                        "Text copied to clipboard",
                                      )
                                    })
                                },
                              },
                              { text: "Close", task: () => {} },
                            ],
                          )
                        }}
                      >
                        <IconCopyRegular />
                      </Button>
                    </div>
                  {/snippet}
                </Overlay>
              </div>
            {/each}
            <TemplatesChoose
              set={(t) => {
                manager.addTemplate(t)
                close()
              }}
            />
            <div class="preset-actions">
              <Overlay
                triggerStyle="width: 100%;min-width: 0;"
                onStateChange={(isOpen) => {
                  if (isOpen) jsonPreset = ""
                }}
              >
                {#snippet trigger()}
                  <Button style="width: 100%;box-sizing: border-box;">
                    Import
                  </Button>
                {/snippet}
                {#snippet overlay({ close }: { close: () => void })}
                  <div class="new-menu">
                    <TextInput
                      bind:value={jsonPreset}
                      placeholder="JSON Preset"
                    />
                    <Button
                      style="width: 100%;box-sizing: border-box;"
                      onclick={() => {
                        manager.addTemplate(JSON.parse(jsonPreset))
                        close()
                      }}
                    >
                      Create
                    </Button>
                  </div>
                {/snippet}
              </Overlay>
              <Button
                style="width: 42px;align-self: stretch;padding: 0;display: grid;place-items: center;"
                onclick={() => {
                  manager.newPreset()
                  close()
                }}
              >
                <IconPlusRegular />
              </Button>
            </div>
          </div>
        {/snippet}
      </Overlay>
    </div>
  </div>
</nav>

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
    min-width: 320px;
    max-width: min(520px, calc(100vw - 32px));
    max-height: 420px;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    padding: calc(var(--padding) / 2);
    gap: calc(var(--padding) / 2);
  }
  .preset,
  .menu,
  .preset-actions {
    display: flex;
    align-items: center;
    gap: calc(var(--padding) / 2);
    min-width: 0;
  }
  .preset-actions {
    width: 100%;
    align-items: stretch;
  }
  .preset-actions > :global(.overlay-root) {
    flex: 1 1 auto;
    min-width: 0;
  }
  .preset > :global(button:first-child) {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .menu,
  .new-menu,
  .bell-menu {
    padding: calc(var(--padding) / 2);
  }
  .new-menu,
  .bell-menu {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--padding) / 2);
  }
  .new-menu {
    width: min(360px, calc(100vw - 32px));
    overflow-x: hidden;
    align-items: stretch;
    box-sizing: border-box;
  }
  .new-menu :global(input) {
    width: 100%;
    box-sizing: border-box;
    min-width: 0;
  }
  .bell-menu {
    min-width: 300px;
    max-width: 420px;
  }
  .version-warning {
    border: 1px solid color-mix(in srgb, var(--danger) 65%, transparent);
    border-radius: 0.4rem;
    padding: 0.38rem 0.75rem;
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: #e7b3ae;
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
    color: #bfd7b7;
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
    color: #e7b3ae;
  }
  .update-btn.error:hover {
    background: color-mix(in srgb, var(--danger) 18%, transparent);
  }
  .topbar-icon {
    width: 28px;
    height: 36px;
    display: grid;
    place-items: center;
    color: var(--textMuted);
    font-size: 1.3rem;
  }
  .topbar-icon:hover {
    color: var(--text);
  }
  .topbar-icon.online {
    color: #cfe0c8;
  }
  nav {
    background-color: var(--bgMedium);
    display: flex;
    align-items: stretch;
    border: 1px solid var(--border);
    border-radius: 0.55rem;
    margin: 0.5rem;
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
  .right-actions {
    display: flex;
    align-items: center;
    gap: calc(var(--padding) / 2);
    padding: 0 var(--padding);
  }
</style>
