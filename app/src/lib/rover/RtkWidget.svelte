<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import DeviceStatus from "./DeviceStatus.svelte"
  const device = $derived(rover.status?.devices.rtk)
  const fix = $derived(device?.data.fix)
  const age = $derived(fix ? Math.max(0, (rover.status?.time ?? 0) - fix.received_at) : null)
  const names = ["No Fix", "GPS", "DGPS", "PPS", "RTK Fixed", "RTK Float", "Dead reckoning", "Manual", "Simulation"]
</script>
<section>
  <h3>RTK position</h3><DeviceStatus {device} />
  <p class:stale={age !== null && age > 3}>{names[fix?.quality ?? 0]} · {age === null ? "No fix received" : `${age.toFixed(1)} s old`}</p>
  <dl><dt>Latitude</dt><dd>{fix?.latitude?.toFixed(8) ?? "—"}°</dd><dt>Longitude</dt><dd>{fix?.longitude?.toFixed(8) ?? "—"}°</dd>
    <dt>Altitude MSL</dt><dd>{fix?.altitude_msl?.toFixed(2) ?? "—"} m</dd><dt>Satellites / HDOP</dt><dd>{fix?.satellites ?? "—"} / {fix?.hdop ?? "—"}</dd>
    <dt>Estimated σ (heuristic)</dt><dd>{fix?.latitude_std?.toFixed(3) ?? "—"} m</dd>
    <dt>NTRIP</dt><dd>{device?.data.ntrip_connected ? "Connected" : "Disconnected / disabled"}</dd>
    <dt>Correction bytes</dt><dd>{device?.data.correction_bytes ?? 0}</dd><dt>NMEA bytes / errors</dt><dd>{device?.data.bytes_read ?? 0} / {device?.data.parse_errors ?? 0}</dd>
  </dl>
  {#if device?.data.ntrip_error}<p class="stale">{device.data.ntrip_error}</p>{/if}
</section>
<style>section { padding: 1rem; height: 100%; overflow: auto; }h3 { margin: 0 0 .5rem; }dl { display: grid; grid-template-columns: 1fr 1fr; gap: .5rem; font-size: .85rem; }dd { margin: 0; font-variant-numeric: tabular-nums; }.stale { color: #f99; }</style>
