<script lang="ts">
  import { global } from "$lib"
  import Button from "$lib/ui/Button.svelte"
  import type { Notification } from "$lib/types"
  import { fly } from "svelte/transition"

  let date = $state(Date.now())

  // The clock only runs while a toast is inside its 3 s life, so an idle app does no work here.
  const TOAST_LIFE_MS = 3000
  $effect(() => {
    const newest = Math.max(0, ...global.notifications.map((it) => it.timestamp))
    if (Date.now() - newest > TOAST_LIFE_MS) return
    date = Date.now()
    const interval = setInterval(() => {
      date = Date.now()
      if (date - newest > TOAST_LIFE_MS) clearInterval(interval)
    }, 250)
    return () => {
      clearInterval(interval)
    }
  })

  function getSimple(notifications: Notification[]) {
    return notifications.filter((it) => it.actions.length <= 0)
  }
  function getActions(notifications: Notification[]) {
    return notifications
      .filter((it) => it.actions.length > 0)
      .filter((it) => !it.executedAction)
  }
</script>

<div class="toasts">
  <section>
    {#each getSimple(global.notifications).filter((it) => date - it.timestamp <= 3000) as notif}
      <div
        class="notif"
        in:fly={{ y: -10, opacity: 0, duration: 200 }}
        out:fly={{ y: -10, opacity: 0, duration: 300 }}
      >
        <p>{notif.text}</p>
      </div>
    {/each}
  </section>
  <section>
    {#each getActions(global.notifications) as notif}
      <div
        class="notif"
        in:fly={{ y: -10, opacity: 0, duration: 200 }}
        out:fly={{ y: -10, opacity: 0, duration: 300 }}
      >
        <p class="center">{notif.text}</p>
        <div class="flex">
          {#each notif.actions as action}
            <Button
              onclick={() => {
                action.task()
                global.notificationsManager.markActionExecuted(notif.id, action.text)
              }}
            >
              {action.text}
            </Button>
          {/each}
        </div>
      </div>
    {/each}
  </section>
</div>

<style>
  /* Pinned to the bottom-right so the newest message sits in the corner and the widget headers
     at the top stay clear. Phones and short landscape screens get a full-width strip at the bottom. */
  .toasts {
    position: absolute;
    right: 0.75rem;
    bottom: 0.75rem;
    z-index: 100;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.35rem;
    width: min(340px, calc(100vw - 24px));
    max-height: 45%;
    overflow: hidden;
    text-align: left;
    pointer-events: none;
  }
  @media (max-width: 520px) {
    .toasts {
      left: 0.75rem;
      right: 0.75rem;
      width: auto;
      max-height: 40%;
    }
  }
  section {
    display: contents;
  }
  .notif {
    width: 100%;
    border: 1px solid var(--bgDark);
    background-color: var(--bgMedium);
    padding: 0.5em 1em;
    border-radius: 1rem;
    pointer-events: auto;
  }
  p {
    margin: 0.25em 0;
  }
  p.center {
    text-align: center;
  }
  .flex {
    display: flex;
    gap: 1rem;
    justify-content: space-between;
  }
</style>
