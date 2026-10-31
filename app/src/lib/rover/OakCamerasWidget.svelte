<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import { hardware } from "$lib/hardware.svelte"
  import DeviceStatus from "./DeviceStatus.svelte"
  import OakCameraView from "./OakCameraView.svelte"
  const device = $derived(rover.status?.devices.oak)
</script>

<section>
  <header><strong>OAK-D Cameras</strong><DeviceStatus {device} /></header>
  <div class="cameras">
    {#each [0, 1, 2] as slot}
      <OakCameraView {slot} name={hardware.config?.oak?.cameras[slot]?.name || `OAK-D ${slot + 1}`}
        camera={device?.data.cameras[slot]} enabled={Boolean(device?.enabled)} simulated={Boolean(device?.simulated)} />
    {/each}
  </div>
</section>

<style>
  section { height: 100%; min-height: 0; display: flex; flex-direction: column; gap: .6rem; padding: .7rem; container-type: inline-size; }
  header { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: .4rem; }
  .cameras { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); grid-auto-rows: minmax(180px, 1fr); gap: .6rem; flex: 1; min-height: 0; overflow: auto; }
  @container (max-width: 700px) { .cameras { grid-template-columns: 1fr; } }
</style>
