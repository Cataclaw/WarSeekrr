<script lang="ts">
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";

  let {
    distance,
    min,
    max,
  }: { distance: number | null; min: number; max: number } = $props();

  const span = $derived(max * 1.12);
  const pct = (m: number) => Math.min(100, Math.max(0, (m / span) * 100));

  const marker = new Tween(0, { duration: 450, easing: cubicOut });
  $effect(() => {
    if (distance != null) marker.target = pct(distance);
  });
</script>

<div class="bar">
  <div class="track">
    <div class="envelope" style:left="{pct(min)}%" style:width="{pct(max) - pct(min)}%"></div>
    {#if distance != null}
      <div class="marker" style:left="{marker.current}%"></div>
    {/if}
  </div>
  <div class="scale num">
    <span style:left="{pct(min)}%">{min}</span>
    <span style:left="{pct(max)}%">{max}</span>
  </div>
</div>

<style>
  .bar {
    padding: 6px 0 14px;
  }
  .track {
    position: relative;
    height: 6px;
    background: var(--surface-2);
    border-radius: 3px;
  }
  .envelope {
    position: absolute;
    top: 0;
    bottom: 0;
    background: color-mix(in srgb, var(--teal) 28%, transparent);
    border-left: 1px solid var(--teal);
    border-right: 1px solid var(--teal);
  }
  .marker {
    position: absolute;
    top: -5px;
    width: 2px;
    height: 16px;
    margin-left: -1px;
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .scale {
    position: relative;
    font-size: 10px;
    color: var(--muted);
  }
  .scale span {
    position: absolute;
    top: 4px;
    transform: translateX(-50%);
  }
</style>
