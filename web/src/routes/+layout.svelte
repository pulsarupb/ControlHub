<script lang="ts">
  import "./global.css"
  import NotificationsUi from "$lib/NotificationsUI.svelte"
  let { children }: { children?: import("svelte").Snippet } = $props()
</script>

<NotificationsUi />
{@render children?.()}
