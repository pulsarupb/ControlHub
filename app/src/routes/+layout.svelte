<script lang="ts">
  import "./global.css"
  import NotificationsUi from "$lib/NotificationsUI.svelte"
  import SwitchNavigation from "$lib/SwitchNavigation.svelte"
  let { children }: { children?: import("svelte").Snippet } = $props()
</script>

<NotificationsUi />
<SwitchNavigation />
{@render children?.()}
