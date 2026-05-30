<script lang="ts">
  import { roverConnection } from "$lib/rover-connection.svelte"

  declare const __APP_VERSION__: string

  let dismissed = $state(false)

  let show = $derived(
    !dismissed && roverConnection.serverVersion !== null && roverConnection.serverVersion !== __APP_VERSION__
  )

  function dismiss() {
    dismissed = true
  }
</script>

{#if show}
  <div class="disclaimer" role="alert">
    <span>
      ⚠ Version mismatch: App v{__APP_VERSION__} — Server v{roverConnection.serverVersion}.
      Please update the app to match the server.
    </span>
    <button onclick={dismiss} aria-label="Dismiss">✕</button>
  </div>
{/if}

<style>
  .disclaimer {
    position: sticky;
    top: 0;
    z-index: 9999;
    background: #991b1b;
    color: #fff;
    padding: 10px 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    font-size: 14px;
    font-weight: 400;
  }
  button {
    background: none;
    border: none;
    color: inherit;
    font-size: 18px;
    cursor: pointer;
    padding: 0 6px;
    line-height: 1;
    opacity: 0.7;
    flex-shrink: 0;
  }
  button:hover {
    opacity: 1;
  }
</style>
