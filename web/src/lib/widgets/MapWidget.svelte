<script lang="ts">
  import { rover } from "$lib/data/rover.svelte"
</script>

<div class="map">
  <div class="grid"></div>
  <div class="rover" style="rotate: {rover.heading}deg">▲</div>
  <p>Heading {rover.heading.toFixed(0)} deg</p>
</div>

<style>
  .map {
    position: relative;
    min-height: 100%;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 0.55rem;
    background: var(--bgDark);
  }
  .grid {
    position: absolute;
    inset: 0;
    background-image: linear-gradient(rgba(255,255,255,.07) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.07) 1px, transparent 1px);
    background-size: 32px 32px;
  }
  .rover {
    position: absolute;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    color: var(--warning);
    font-size: 2rem;
  }
  p {
    position: absolute;
    left: 1rem;
    bottom: 1rem;
    margin: 0;
  }
</style>
