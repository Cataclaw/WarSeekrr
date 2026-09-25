<script lang="ts">
  import type { Coord, PointKind } from "./api";

  let {
    kind,
    hotkey,
    coord,
    flash = false,
    countdown = null,
    onchange,
    oncapture,
  }: {
    kind: PointKind;
    hotkey: string;
    coord: Coord | null;
    flash?: boolean;
    countdown?: number | null;
    onchange: (c: Coord | null) => void;
    oncapture: () => void;
  } = $props();

  let x = $state("");
  let y = $state("");
  let row: HTMLElement;

  $effect(() => {
    const c = coord;
    if (row?.contains(document.activeElement)) return;
    x = c ? c.x.toFixed(2) : "";
    y = c ? c.y.toFixed(2) : "";
  });

  function commit() {
    const nx = parseFloat(x.replace(",", "."));
    const ny = parseFloat(y.replace(",", "."));
    if (x.trim() === "" && y.trim() === "") onchange(null);
    else if (Number.isFinite(nx) && Number.isFinite(ny)) onchange({ x: nx, y: ny });
  }

  function paste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData("text") ?? "";
    const xm = text.match(/x\s*[:=]?\s*(\d+(?:[.,]\d+)?)/i);
    const ym = text.match(/y\s*[:=]?\s*(\d+(?:[.,]\d+)?)/i);
    const nums = text.match(/\d+(?:[.,]\d+)?/g);
    const pair = xm && ym ? [xm[1], ym[1]] : nums?.length === 2 ? nums : null;
    if (!pair) return;
    e.preventDefault();
    [x, y] = pair.map((n) => n.replace(",", "."));
    commit();
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
    if (e.key === "Escape") {
      x = coord ? coord.x.toFixed(2) : "";
      y = coord ? coord.y.toFixed(2) : "";
      (e.target as HTMLInputElement).blur();
    }
  }
</script>

<div class="row {kind}" class:flash class:empty={!coord} bind:this={row}>
  <span class="tag" title={kind === "artillery" ? "Mortar position" : "Target"}>
    {kind === "artillery" ? "A" : "T"}
  </span>

  <label>
    <span class="axis">X</span>
    <input class="num" bind:value={x} onchange={commit} onpaste={paste} onkeydown={key} placeholder="--.--" inputmode="decimal" />
  </label>
  <label>
    <span class="axis">Y</span>
    <input class="num" bind:value={y} onchange={commit} onpaste={paste} onkeydown={key} placeholder="--.--" inputmode="decimal" />
  </label>

  <button class="key num" onclick={oncapture} title="Capture at cursor in 3 s (hotkey: {hotkey})">
    {countdown ?? hotkey}
  </button>
  <button class="clear" onclick={() => onchange(null)} disabled={!coord} title="Clear">×</button>
</div>

<style>
  .row {
    --tone: var(--teal);
    display: grid;
    grid-template-columns: 22px 1fr 1fr auto 18px;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid var(--hair);
    border-radius: 4px;
    background: var(--surface);
    transition: border-color 0.25s, background 0.25s;
  }
  .row.target {
    --tone: var(--cyan);
  }
  .row.flash {
    border-color: var(--cyan);
    background: color-mix(in srgb, var(--cyan) 10%, var(--surface));
    animation: sweep 0.7s ease-out;
  }
  @keyframes sweep {
    from {
      box-shadow: inset 0 0 0 1px var(--cyan), 0 0 14px color-mix(in srgb, var(--cyan) 50%, transparent);
    }
    to {
      box-shadow: inset 0 0 0 0 transparent, 0 0 0 transparent;
    }
  }
  .tag {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border: 1px solid var(--tone);
    color: var(--tone);
    font-weight: 700;
    font-size: 12px;
    border-radius: 3px;
  }
  .empty .tag {
    border-style: dashed;
    color: var(--muted);
    border-color: var(--dim);
  }
  label {
    display: flex;
    align-items: baseline;
    gap: 5px;
    min-width: 0;
    border-bottom: 1px solid transparent;
  }
  label:focus-within {
    border-bottom-color: var(--tone);
  }
  .axis {
    font-size: 10px;
    color: var(--muted);
  }
  input {
    all: unset;
    width: 100%;
    min-width: 0;
    font-family: var(--font-num);
    font-size: 16px;
    color: var(--text);
    user-select: text;
  }
  input::placeholder {
    color: var(--dim);
  }
  .key {
    font-size: 10px;
    min-width: 30px;
    padding: 3px 6px;
    color: var(--muted);
    border: 1px solid var(--hair-strong);
    border-radius: 3px;
    text-align: center;
  }
  .key:hover {
    color: var(--tone);
    border-color: var(--tone);
  }
  .clear {
    color: var(--muted);
    font-size: 15px;
    line-height: 1;
  }
  .clear:hover:not(:disabled) {
    color: var(--alarm);
  }
</style>
