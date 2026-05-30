<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window"

  let maximized = $state(false)
  let available = $state(true)

  let appWindow: ReturnType<typeof getCurrentWindow>

  try {
    appWindow = getCurrentWindow()
  } catch {
    available = false
  }

  async function toggleMaximize() {
    if (!appWindow) return
    await appWindow.toggleMaximize()
    maximized = await appWindow.isMaximized()
  }

  async function minimize() {
    if (!appWindow) return
    await appWindow.minimize()
  }

  async function close() {
    if (!appWindow) return
    await appWindow.close()
  }

  async function refresh() {
    if (!appWindow) return
    maximized = await appWindow.isMaximized()
  }
</script>

<svelte:window on:resize={refresh} />

{#if available}
  <div class="window-controls">
    <button class="control minimize" onclick={minimize} aria-label="Minimize">
      <svg viewBox="0 0 12 12" width="12" height="12">
        <rect x="1" y="5.5" width="10" height="1" fill="currentColor" />
      </svg>
    </button>
    <button class="control maximize" onclick={toggleMaximize} aria-label="Maximize">
      {#if maximized}
        <svg viewBox="0 0 12 12" width="12" height="12">
          <rect x="2.5" y="0.5" width="9" height="9" rx="0.5" fill="none" stroke="currentColor" stroke-width="1" />
          <rect x="0.5" y="2.5" width="9" height="9" rx="0.5" fill="none" stroke="currentColor" stroke-width="1" />
        </svg>
      {:else}
        <svg viewBox="0 0 12 12" width="12" height="12">
          <rect x="1" y="1" width="10" height="10" rx="0.5" fill="none" stroke="currentColor" stroke-width="1" />
        </svg>
      {/if}
    </button>
    <button class="control close" onclick={close} aria-label="Close">
      <svg viewBox="0 0 12 12" width="12" height="12">
        <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" stroke-width="1.2" />
        <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" stroke-width="1.2" />
      </svg>
    </button>
  </div>
{/if}

<style>
  .window-controls {
    display: flex;
    align-items: center;
    gap: 0;
  }
  .control {
    all: unset;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 42px;
    height: 100%;
    color: var(--text);
    transition: background 0.1s;
  }
  .control:hover {
    background: var(--bgLight);
  }
  .close:hover {
    background: #e81123;
    color: #fff;
  }
</style>
