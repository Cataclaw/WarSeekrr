<script lang="ts">
  import { untrack } from "svelte";
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";

  let {
    azimuth,
    mil,
    distance,
    table,
  }: {
    azimuth: number | null;
    mil: number | null;
    distance: number | null;
    table: [number, number][] | null;
  } = $props();

  const CX = 180;
  const CY = 170;
  const DEG_PX = 4.5;
  const AZ_HALF = 31;
  const MIL_PX = 0.9;
  const MIL_HALF = 84;

  const az = new Tween(0, { duration: 550, easing: cubicOut });
  const el = new Tween(400, { duration: 550, easing: cubicOut });

  $effect(() => {
    if (azimuth == null) return;
    // Shortest way round, so 359 -> 1 doesn't scroll the whole tape.
    const from = untrack(() => az.target);
    az.target = from + ((((azimuth - from) % 360) + 540) % 360) - 180;
  });
  $effect(() => {
    if (mil != null) el.target = mil;
  });

  const norm = (d: number) => ((Math.round(d * 10) / 10) % 360 + 360) % 360;
  const pad3 = (d: number) => String(Math.round(norm(d)) % 360).padStart(3, "0");

  const azTicks = $derived.by(() => {
    const out: { x: number; kind: "major" | "mid" | "minor"; label?: string; fade: number }[] = [];
    const first = Math.ceil((az.current - AZ_HALF) / 2.5) * 2.5;
    for (let d = first; d <= az.current + AZ_HALF; d += 2.5) {
      const off = d - az.current;
      const n = ((d % 360) + 360) % 360;
      const kind = n % 15 === 0 ? "major" : n % 7.5 === 0 ? "mid" : "minor";
      out.push({
        x: CX + off * DEG_PX,
        kind,
        label: kind === "major" ? pad3(n) : undefined,
        fade: 1 - (Math.abs(off) / AZ_HALF) * 0.75,
      });
    }
    return out;
  });

  /** Range the table gives for an elevation; tables are sorted by range, mil falling. */
  function rangeAt(m: number): number | null {
    if (!table) return null;
    for (let i = 0; i < table.length - 1; i++) {
      const [r0, m0] = table[i];
      const [r1, m1] = table[i + 1];
      const lo = Math.min(m0, m1);
      const hi = Math.max(m0, m1);
      if (m >= lo && m <= hi && m0 !== m1) return r0 + ((m - m0) / (m1 - m0)) * (r1 - r0);
    }
    return null;
  }

  const milTicks = $derived.by(() => {
    const out: { y: number; major: boolean; mil: number; range: number | null; fade: number }[] = [];
    const first = Math.ceil((el.current - MIL_HALF) / 10) * 10;
    for (let m = first; m <= el.current + MIL_HALF; m += 10) {
      if (m < 0) continue;
      const off = m - el.current;
      const major = m % 50 === 0;
      out.push({
        y: CY + off * MIL_PX,
        major,
        mil: m,
        range: major ? rangeAt(m) : null,
        fade: 1 - (Math.abs(off) / MIL_HALF) * 0.8,
      });
    }
    return out;
  });

  const corners = [
    "M 8 22 V 8 H 22",
    "M 338 8 H 352 V 22",
    "M 8 268 V 282 H 22",
    "M 338 282 H 352 V 268",
  ];
</script>

<svg viewBox="0 0 360 290" class="sight" class:idle={azimuth == null} role="img" aria-label="Mortar sight">
  <defs>
    <clipPath id="az-window"><rect x="36" y="14" width="288" height="50" /></clipPath>
    <clipPath id="mil-window"><rect x="0" y="104" width="360" height="156" /></clipPath>
  </defs>

  {#each corners as d}<path {d} class="corner" />{/each}

  <g clip-path="url(#az-window)">
    {#each azTicks as t}
      <g opacity={t.fade}>
        <line x1={t.x} x2={t.x} y1={36} y2={36 + (t.kind === "major" ? 16 : t.kind === "mid" ? 12 : 7)} class="tick" class:strong={t.kind !== "minor"} />
        {#if t.label}<text x={t.x} y={28} class="az-label">{t.label}</text>{/if}
      </g>
    {/each}
  </g>
  <path d="M {CX} 56 l -5 8 h 10 z" class="marker" />
  <text x={CX} y={80} class="az-value">{azimuth == null ? "---.-" : norm(az.current).toFixed(1)}°</text>

  <line x1={72} x2={288} y1={CY} y2={CY} class="guide" />

  {#if mil != null}
    <g clip-path="url(#mil-window)">
      {#each milTicks as t}
        <g opacity={t.fade}>
          <line x1={t.major ? 290 : 296} x2={304} y1={t.y} y2={t.y} class="tick" class:strong={t.major} />
          {#if t.major}
            {@const clear = Math.abs(t.y - CY) > 18}
            {#if clear}<text x={310} y={t.y} class="scale-label">{t.mil}</text>{/if}
            {#if t.range != null}
              <line x1={56} x2={70} y1={t.y} y2={t.y} class="tick strong" />
              {#if clear}<text x={52} y={t.y} class="scale-label end">{Math.round(t.range)}M</text>{/if}
            {/if}
          {:else}
            <line x1={56} x2={62} y1={t.y} y2={t.y} class="tick" />
          {/if}
        </g>
      {/each}
    </g>

    <g class="badge">
      <rect x={288} y={CY - 11} width={62} height={22} rx="2" />
      <text x={319} y={CY}>{Math.round(el.current)}</text>
    </g>
  {/if}
  {#if distance != null}
    <g class="badge">
      <rect x={6} y={CY - 11} width={64} height={22} rx="2" />
      <text x={38} y={CY}>{Math.round(distance)}M</text>
    </g>
  {/if}

  <text x={30} y={96} class="axis">RNG</text>
  <text x={330} y={96} class="axis">MIL</text>

  <g class="reticle">
    <path d="M {CX - 26} {CY - 22} h -6 v 44 h 6 M {CX + 26} {CY - 22} h 6 v 44 h -6" />
    <circle cx={CX} cy={CY} r="4" />
    <path d="M {CX} {CY - 58} v 18 M {CX - 12} {CY - 40} h 24" />
    <path d="M {CX} {CY + 58} v -18 M {CX - 12} {CY + 40} h 24" />
  </g>

  {#if azimuth == null}
    <text x={CX} y={CY + 84} class="idle-text">AWAITING FIX</text>
  {:else if mil == null}
    <text x={CX} y={CY + 84} class="idle-text alarm">OUT OF RANGE</text>
  {/if}
</svg>

<style>
  .sight {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
  }
  .corner {
    fill: none;
    stroke: var(--hair-strong);
    stroke-width: 1.5;
  }
  .tick {
    stroke: var(--muted);
    stroke-width: 1;
  }
  .tick.strong {
    stroke: var(--text);
    stroke-width: 1.5;
  }
  .az-label,
  .scale-label,
  .axis,
  .idle-text {
    font-family: var(--font-ui);
    fill: var(--text);
  }
  .az-label {
    font-size: 11px;
    text-anchor: middle;
    letter-spacing: 0.05em;
  }
  .scale-label {
    font-size: 11px;
    letter-spacing: 0.08em;
    dominant-baseline: central;
  }
  .scale-label.end {
    text-anchor: end;
  }
  .axis {
    font-size: 10px;
    letter-spacing: 0.2em;
    text-anchor: middle;
    fill: var(--muted);
  }
  .marker {
    fill: var(--accent);
    filter: drop-shadow(0 0 4px var(--accent));
  }
  .az-value {
    font: 600 15px var(--font-num);
    fill: var(--accent);
    text-anchor: middle;
  }
  .guide {
    stroke: var(--accent);
    stroke-width: 1;
    stroke-dasharray: 2 5;
    opacity: 0.5;
  }
  .badge rect {
    fill: var(--accent);
    filter: drop-shadow(0 0 6px color-mix(in srgb, var(--accent) 60%, transparent));
  }
  .badge text {
    font: 700 13px var(--font-num);
    fill: var(--bg);
    text-anchor: middle;
    dominant-baseline: central;
  }
  .reticle path,
  .reticle circle {
    fill: none;
    stroke: var(--text);
    stroke-width: 2;
  }
  .idle-text.alarm {
    fill: var(--alarm);
  }
  .idle .tick,
  .idle .az-label {
    opacity: 0.35;
  }
  .idle-text {
    font-size: 10px;
    letter-spacing: 0.3em;
    text-anchor: middle;
    fill: var(--muted);
  }
</style>
