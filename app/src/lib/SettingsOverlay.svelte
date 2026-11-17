<script lang="ts">
  import Portal from "svelte-portal"
  import { friendlyError } from "$lib/friendly-error"
  import Button from "$lib/ui/Button.svelte"
  import ConfigFields from "$lib/ConfigFields.svelte"
  import { getConfig, updateConfig, type FullConfig } from "$lib/config-api"
  import { hardware, releaseArm } from "$lib/hardware.svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { releaseGamepad } from "$lib/gamepad-control.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"
  import { THEMES, theme, type ThemeId } from "$lib/theme.svelte"

  let { show = false, onclose }: { show?: boolean; onclose?: () => void } = $props()
  type Section = "general" | keyof FullConfig
  let config: FullConfig | null = $state(null)
  let active: Section = $state("general")
  let error = $state("")
  let saving = $state(false)
  const devices: [keyof FullConfig, string][] = [["drive", "Motors"], ["kinova", "Kinova"], ["rtk", "RTK / NTRIP"], ["zed", "ZED"], ["oak", "OAK-D Cameras"], ["gamepad", "Gamepad"]]
  const notes: Record<keyof FullConfig, string> = {
    drive: "Ports: 0 = A, 1 = B. Power limit is 1–1000, not wheel speed.",
    kinova: "Speeds in m/s and deg/s. Targets are in base-frame meters and degrees.",
    rtk: "Corrections go to the GNSS receiver only, never to ZED.",
    zed: "Mount values place the camera in the rover base (meters, degrees). Mark it configured only after measuring.",
    oak: "Blank MXID finds cameras in sorted order. Enter full IDs to keep each camera in its slot after restarts.",
    gamepad: "Standard Gamepad API button and axis indices.",
  }
  const deviceLabel = $derived(devices.find(([key]) => key === active)?.[1] ?? "")

  let loadToken = 0
  function load() {
    const token = ++loadToken
    config = null
    error = ""
    getConfig()
      .then(value => { if (token === loadToken) config = value })
      .catch(e => { if (token === loadToken) error = e instanceof TypeError ? "Rover unreachable" : e instanceof Error ? e.message : friendlyError(e) })
  }
  $effect(() => {
    if (!show) return
    load()
    return () => { loadToken++ }
  })
  async function save() {
    if (!config || active === "general") return
    saving = true
    error = ""
    try {
      releaseArm()
      rover.releaseJoystick()
      releaseGamepad()
      await updateConfig(config)
      hardware.config = config
      onclose?.()
    } catch (e) { error = friendlyError(e) }
    finally { saving = false }
  }
  function handleKeydown(e: KeyboardEvent) {
    if (show && e.key === "Escape") onclose?.()
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if show}
  <Portal>
    <div class="backdrop" role="presentation" onclick={() => onclose?.()}></div>
    <div class="settings" role="dialog" aria-modal="true" aria-label="Settings">
      <header>
        <h2>Settings</h2>
        <button class="close" onclick={() => onclose?.()} aria-label="Close settings">✕</button>
      </header>
      <div class="layout">
        <nav aria-label="Settings sections">
          <button class:active={active === "general"} aria-current={active === "general" ? "page" : undefined} onclick={() => (active = "general")}>General</button>
          <span class="group-label">Devices</span>
          {#each devices as [key, label]}
            <button class:active={active === key} aria-current={active === key ? "page" : undefined} onclick={() => (active = key)}>{label}</button>
          {/each}
        </nav>

        <main>
          {#if active === "general"}
            <section class="panel">
              <h3>Theme</h3>
              <p class="note">Changes the colours across the whole app. Status colours and layout stay the same. Saved on this device.</p>
              <div class="themes" role="group" aria-label="Themes">
                {#each THEMES as t}
                  <button class="theme" class:selected={theme.current === t.id} aria-pressed={theme.current === t.id} onclick={() => theme.apply(t.id as ThemeId)}>
                    <span class="swatch" data-theme={t.id} aria-hidden="true">
                      <span class="sw-bg">
                        <span class="sw-bar"></span>
                        <span class="sw-card">
                          <i></i><i></i>
                        </span>
                        <span class="sw-dots"><b></b><b></b></span>
                      </span>
                    </span>
                    <span class="theme-name">{t.name}</span>
                    <span class="theme-note">{t.note}</span>
                  </button>
                {/each}
              </div>
            </section>
          {:else}
            <section class="panel">
              <h3>{deviceLabel}</h3>
              {#if error}
                <p role="alert">Could not load configuration: {error}. Check the rover connection, then retry.</p>
                <div><Button onclick={load}>Retry</Button></div>
              {/if}
              {#if config}
                <p class="note">{notes[active]}</p>
                <ConfigFields value={config[active]} />
                <footer><Button onclick={() => onclose?.()}>Cancel</Button><Button onclick={save} disabled={saving}>{saving ? "Saving…" : "Save"}</Button></footer>
              {:else if !error && roverConnection.state === "online"}<p role="status">Loading configuration…</p>
              {:else if !error}<p role="status">The rover is offline. Connect to it to view and change these settings.</p>{/if}
            </section>
          {/if}
        </main>
      </div>
    </div>
  </Portal>
{/if}

<style>
  /* A modal over the workspace, not a page: the backdrop dims the grid and a click outside closes it. */
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 1100;
    background: rgba(0, 0, 0, 0.55);
  }
  .settings {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 1101;
    display: flex;
    flex-direction: column;
    width: min(880px, calc(100vw - 32px));
    height: min(620px, calc(100vh - 32px));
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    background: var(--bgDark);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    color: var(--text);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    min-height: 3.25rem;
    padding: 0 1rem;
    border-bottom: 1px solid var(--border);
    background: var(--bgMedium);
  }
  h2 { margin: 0; font-size: 1.05rem; font-weight: 600; }
  .close {
    all: unset;
    cursor: pointer;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 0.35rem;
    color: var(--textMuted);
  }
  .close:hover { color: var(--text); background: var(--bgLight); }
  .close:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }
  .layout { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(150px, 220px) minmax(0, 1fr); }
  nav {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.75rem;
    overflow-y: auto;
    border-right: 1px solid var(--border);
    background: var(--bgMedium);
  }
  nav button {
    all: unset;
    box-sizing: border-box;
    position: relative;
    padding: 0.5rem 0.7rem 0.5rem 1.05rem;
    border-radius: 0.4rem;
    color: var(--textMuted);
    font-size: 0.9rem;
    cursor: pointer;
  }
  nav button:hover { color: var(--text); background: var(--bgLight); }
  /* Selected is marked by weight, colour and a small accent dot, not by a filled box. */
  nav button.active { color: var(--text); font-weight: 600; }
  nav button.active::before {
    content: "";
    position: absolute;
    left: 0.4rem;
    top: 50%;
    width: 0.35rem;
    height: 0.35rem;
    margin-top: -0.175rem;
    border-radius: 50%;
    background: var(--primary);
  }
  nav button:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }
  .group-label {
    margin: 0.9rem 0.7rem 0.3rem;
    font-size: 0.7rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--textSubtle);
  }
  main { min-width: 0; min-height: 0; overflow-y: auto; }
  .panel { max-width: 880px; padding: 1.25rem 1.5rem 2rem; display: flex; flex-direction: column; gap: 0.9rem; }
  h3 { margin: 0; font-size: 1.1rem; font-weight: 600; }
  .note { margin: 0; font-size: 0.85rem; line-height: 1.5; color: var(--textMuted); }
  [role="alert"] { margin: 0; font-size: 0.85rem; color: var(--danger); }
  footer { display: flex; justify-content: flex-end; gap: 0.5rem; padding-top: 0.6rem; border-top: 1px solid var(--border); }

  /* Each card paints its own theme: the data-theme attribute on the swatch sets the variables for its children. */
  .themes { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 0.75rem; }
  .theme {
    all: unset;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.55rem;
    border: 1px solid var(--border);
    border-radius: 0.6rem;
    background: var(--bgMedium);
    cursor: pointer;
  }
  .theme:hover { border-color: var(--borderStrong); }
  .theme.selected { border-color: var(--primary); box-shadow: 0 0 0 1px var(--primary); }
  .theme:focus-visible { outline: 2px solid var(--primary); outline-offset: 3px; }
  .theme-name { font-size: 0.9rem; font-weight: 600; color: var(--text); }
  .theme-note { font-size: 0.75rem; color: var(--textMuted); }
  .swatch { display: block; border-radius: 0.4rem; overflow: hidden; border: 1px solid var(--border); }
  .sw-bg { position: relative; display: block; height: 84px; background: var(--bgDark); }
  .sw-bar { position: absolute; left: 0; right: 0; top: 0; height: 14px; background: var(--bgMedium); }
  .sw-card {
    position: absolute;
    left: 10px;
    right: 34px;
    top: 26px;
    bottom: 12px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 7px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--surfaceRaised);
  }
  .sw-card i { display: block; height: 4px; border-radius: 2px; background: var(--textMuted); }
  .sw-card i:first-child { width: 60%; background: var(--text); }
  .sw-card i:last-child { width: 36%; }
  .sw-dots { position: absolute; right: 8px; top: 26px; display: flex; flex-direction: column; gap: 6px; }
  .sw-dots b { display: block; width: 14px; height: 14px; border-radius: 50%; background: var(--primary); }
  .sw-dots b:last-child { background: var(--accent); }

  @media (max-width: 640px) {
    .layout { grid-template-columns: minmax(120px, 150px) minmax(0, 1fr); }
    .panel { padding: 1rem; }
  }
</style>
