<script lang="ts">
  import { global } from "$lib"
  import Button from "$lib/ui/Button.svelte"
  import type { Notification } from "$lib/types"
  import { fly } from "svelte/transition"

  let date = $state(Date.now())

  $effect(() => {
    const interval = setInterval(() => {
      date = Date.now()
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
  .toasts {
    position: absolute;
    top: var(--padding);
    left: 50%;
    transform: translateX(-50%);
    z-index: 100;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    width: min(440px, calc(100vw - 32px));
    text-align: center;
    pointer-events: none;
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
