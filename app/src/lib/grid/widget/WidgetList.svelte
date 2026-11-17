<script lang="ts">
  import PreviewCard from "$lib/ui/PreviewCard.svelte"
  import PreviewBox from "../PreviewBox.svelte"
  import WidgetContent from "../WidgetContent.svelte"
  import { getComponents } from "$lib/widgets/registry"

  // Shared by "+" (add a tab) and "Choose" (fill an empty tab) so both show the same picker.
  let {
    onpick,
    close,
  }: {
    onpick: (id: string) => void
    close: () => void
  } = $props()

  // Sections in display order. Each widget is listed under exactly one section.
  const sections: { title: string; ids: string[] }[] = [
    { title: "Drive", ids: ["joystick", "joystick-single", "joystick-throttle", "joystick-steer", "gamepad", "emergency", "motors"] },
    { title: "Navigation", ids: ["minimap", "rtk", "rtk-map"] },
    { title: "Cameras", ids: ["zed-video", "oak-cameras", "d435-camera"] },
    { title: "Arm", ids: ["kinova-controls", "kinova-view"] },
  ]

  const details: Record<string, string> = {
    joystick: "Throttle and steering bars in one pad",
    "joystick-single": "One stick for throttle and steering",
    "joystick-throttle": "Throttle bar, forward and back",
    "joystick-steer": "Steering bar, spin left and right",
    gamepad: "Controller state and button bindings",
    emergency: "Stop the rover and enable drive",
    motors: "Per-wheel power, encoder counts, speed and faults",
    minimap: "ZED camera pose and point-cloud map",
    rtk: "GNSS fix, NTRIP link and NMEA counters",
    "rtk-map": "Offline map with the rover position",
    "zed-video": "ZED color or depth feed",
    "oak-cameras": "Three OAK-D cameras with distance readout",
    "d435-camera": "Gripper camera with optional crosshair",
    "kinova-controls": "Arm enable, stop, jog and gripper",
    "kinova-view": "Arm model and IK target in 3D",
  }

  const widgets = $derived(getComponents("widget"))
  const grouped = $derived(
    sections
      .map((section) => ({
        title: section.title,
        items: section.ids
          .map((id) => widgets.find((w) => w.id === id))
          .filter((w): w is NonNullable<typeof w> => w !== undefined),
      }))
      .filter((section) => section.items.length > 0),
  )
</script>

<div class="chooser">
  {#each grouped as section (section.title)}
    <section>
      <h3>{section.title}</h3>
      <div class="grid">
        {#each section.items as w (w.id)}
          <PreviewCard
            lazy
            name={w.name}
            frame="104px"
            detail="{details[w.id] ?? ''} · min {w.minW ?? 1}&nbsp;×&nbsp;{w.minH ?? 1}"
            onclick={() => {
              close()
              onpick(w.id)
            }}
          >
            <PreviewBox scale={0.5} width="100%" height="104px">
              <WidgetContent widgetID={w.id} />
            </PreviewBox>
          </PreviewCard>
        {/each}
      </div>
    </section>
  {/each}
</div>

<style>
  .chooser {
    width: min(560px, calc(100vw - 32px));
    max-height: min(72vh, 620px);
    overflow-y: auto;
    padding: 0.5rem;
    display: grid;
    gap: 0.9rem;
  }
  h3 {
    margin: 0 0 0.4rem;
    padding: 0 0.15rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--textMuted);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 0.5rem;
  }
</style>
