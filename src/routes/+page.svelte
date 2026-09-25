<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import PointRow from "$lib/PointRow.svelte";
  import RangeBar from "$lib/RangeBar.svelte";
  import Rolling from "$lib/Rolling.svelte";
  import SettingsSheet from "$lib/Settings.svelte";
  import Sight from "$lib/Sight.svelte";
  import {
    captureNow,
    getState,
    onState,
    setPoint,
    swapPoints,
    updateSettings,
    type CaptureReport,
    type PointKind,
    type Settings,
    type Snapshot,
  } from "$lib/api";

  let snap = $state<Snapshot | null>(null);
  let showSettings = $state(false);
  let pinned = $state(false);
  let error = $state("");
  let flash = $state<PointKind | null>(null);
  let failure = $state<CaptureReport | null>(null);
  let countdown = $state<{ kind: PointKind; n: number } | null>(null);

  const weapon = $derived(snap?.weapons.find((w) => w.id === snap?.settings.weaponId));
  const sol = $derived(snap?.solution ?? null);
  const alarm = $derived(sol != null && !sol.inRange);

  let arcIndex = $state(0);
  const arc = $derived(sol?.arcs[Math.min(arcIndex, (sol?.arcs.length ?? 1) - 1)] ?? null);
  const table = $derived(weapon?.arcs.find((a) => a.id === arc?.id)?.table ?? weapon?.arcs[0]?.table ?? null);

  let failTimer: ReturnType<typeof setTimeout>;

  onMount(() => {
    getState().then((s) => (snap = s));
    const un = onState((s) => {
      const cap = s.lastCapture;
      if (cap && JSON.stringify(cap) !== JSON.stringify(snap?.lastCapture)) {
        if (cap.ok) {
          flash = cap.kind;
          failure = null;
          setTimeout(() => (flash = null), 800);
        } else {
          failure = cap;
          clearTimeout(failTimer);
          failTimer = setTimeout(() => (failure = null), 6000);
        }
      }
      snap = s;
    });
    return () => void un.then((f) => f());
  });

  async function run(f: () => Promise<unknown>) {
    error = "";
    try {
      await f();
      return true;
    } catch (e) {
      error = String(e);
      return false;
    }
  }

  const setWeapon = (id: string) => run(() => updateSettings({ ...snap!.settings, weaponId: id }));

  async function saveSettings(s: Settings) {
    if (await run(() => updateSettings(s))) showSettings = false;
  }

  async function togglePin() {
    pinned = !pinned;
    await run(() => getCurrentWindow().setAlwaysOnTop(pinned));
  }

  function delayedCapture(kind: PointKind) {
    countdown = { kind, n: 3 };
    const tick = () => {
      if (!countdown) return;
      if (countdown.n <= 1) {
        countdown = null;
        run(() => captureNow(kind));
      } else {
        countdown = { kind, n: countdown.n - 1 };
        setTimeout(tick, 1000);
      }
    };
    setTimeout(tick, 1000);
  }

  const milSpan = (a: { minMil: number; maxMil: number }) => Math.round(a.minMil) !== Math.round(a.maxMil);
</script>

{#if snap}
  <main class:alarm>
    <header>
      <div class="brand">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="8" r="6.5" />
          <path d="M8 0v4M8 12v4M0 8h4M12 8h4" />
        </svg>
        <span>WARSEEKRR</span>
      </div>

      <div class="seg" role="tablist">
        {#each snap.weapons as w}
          <button
            role="tab"
            aria-selected={w.id === snap.settings.weaponId}
            class:on={w.id === snap.settings.weaponId}
            onclick={() => setWeapon(w.id)}>{w.name.replace(" Mortar", "")}</button
          >
        {/each}
      </div>

      <button class="icon" class:on={pinned} onclick={togglePin} title="Keep on top" aria-label="Keep on top">
        <svg viewBox="0 0 16 16"><path d="M5 2h6l-1 5 2 2H4l2-2zM8 9v5" /></svg>
      </button>
      <button class="icon" onclick={() => (showSettings = true)} title="Settings" aria-label="Settings">
        <svg viewBox="0 0 16 16"><path d="M2 4h12M2 8h12M2 12h12" /><circle cx="5" cy="4" r="1.5" /><circle cx="11" cy="8" r="1.5" /><circle cx="7" cy="12" r="1.5" /></svg>
      </button>
    </header>

    <section class="sight">
      <Sight azimuth={sol?.azimuthDeg ?? null} mil={arc?.minMil ?? null} distance={sol?.distanceM ?? null} {table} />
    </section>

    <section class="readout">
      <div class="cell">
        <span class="label">Azimuth</span>
        <span class="num v">{#if sol}<Rolling value={sol.azimuthDeg} decimals={1} />{:else}---.-{/if}<i>°</i></span>
      </div>
      <div class="cell main">
        <span class="label">
          Elevation
          {#if sol && sol.arcs.length > 1}
            <span class="arcs">
              {#each sol.arcs as a, i}
                <button class:on={a.id === arc?.id} onclick={() => (arcIndex = i)}>{a.name}</button>
              {/each}
            </span>
          {/if}
        </span>
        <span class="num v big">
          {#if arc}
            {#if milSpan(arc)}{Math.round(arc.minMil)}–{Math.round(arc.maxMil)}{:else}<Rolling value={arc.minMil} />{/if}
          {:else}----{/if}<i>mil</i>
        </span>
      </div>
      <div class="cell">
        <span class="label">Range</span>
        <span class="num v">{#if sol}<Rolling value={sol.distanceM} />{:else}---{/if}<i>m</i></span>
      </div>
    </section>

    {#if weapon}
      <section class="envelope">
        <RangeBar distance={sol?.distanceM ?? null} min={weapon.rangeM[0]} max={weapon.rangeM[1]} />
        {#if sol}
          <span class="status num" class:ok={sol.inRange}>● {sol.inRange ? "IN RANGE" : "OUT OF RANGE"}</span>
        {/if}
      </section>
    {/if}

    <section class="points">
      <PointRow
        kind="artillery"
        hotkey={snap.settings.hotkeyArtillery}
        coord={snap.artillery}
        flash={flash === "artillery"}
        countdown={countdown?.kind === "artillery" ? countdown.n : null}
        onchange={(c) => run(() => setPoint("artillery", c))}
        oncapture={() => delayedCapture("artillery")}
      />
      <button class="swap" onclick={() => run(swapPoints)} title="Swap" aria-label="Swap points">⇅</button>
      <PointRow
        kind="target"
        hotkey={snap.settings.hotkeyTarget}
        coord={snap.target}
        flash={flash === "target"}
        countdown={countdown?.kind === "target" ? countdown.n : null}
        onchange={(c) => run(() => setPoint("target", c))}
        oncapture={() => delayedCapture("target")}
      />
    </section>

    {#if !sol && !snap.artillery && !snap.target}
      <p class="hint">
        Open the map, aim the crosshair at your mortar and press <kbd>{snap.settings.hotkeyArtillery}</kbd>,
        then at the target and press <kbd>{snap.settings.hotkeyTarget}</kbd>.
      </p>
    {/if}

    {#if snap.hotkeyError}<p class="error">{snap.hotkeyError}</p>{/if}
    {#if error && !showSettings}<p class="error">{error}</p>{/if}
  </main>

  {#if failure}
    <button class="toast" onclick={() => (failure = null)}>
      {#if failure.image}<img src={failure.image} alt="" />{/if}
      <span>
        <b>No readout found</b>
        Aim so the x/y numbers show beside the crosshair, then press
        {failure.kind === "artillery" ? snap.settings.hotkeyArtillery : snap.settings.hotkeyTarget} again.
      </span>
    </button>
  {/if}

  {#if showSettings}
    <SettingsSheet
      settings={snap.settings}
      lastCapture={snap.lastCapture}
      {error}
      onsave={saveSettings}
      onclose={() => ((showSettings = false), (error = ""))}
    />
  {/if}
{/if}

<style>
  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 10px 12px 12px;
    background:
      radial-gradient(120% 60% at 50% 22%, color-mix(in srgb, var(--accent) 7%, transparent), transparent 70%),
      var(--bg);
  }
  main.alarm {
    --accent: var(--alarm);
  }

  header {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-right: auto;
    font-weight: 700;
    font-size: 13px;
    letter-spacing: 0.24em;
  }
  .brand svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: var(--teal);
    stroke-width: 1.4;
  }
  .seg {
    display: flex;
    border: 1px solid var(--hair-strong);
    border-radius: 3px;
    overflow: hidden;
  }
  .seg button {
    padding: 4px 9px;
    font-size: 11px;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .seg button + button {
    border-left: 1px solid var(--hair-strong);
  }
  .seg button.on {
    background: var(--teal);
    color: var(--bg);
    font-weight: 700;
  }
  .icon {
    width: 26px;
    height: 24px;
    display: grid;
    place-items: center;
    border: 1px solid var(--hair);
    border-radius: 3px;
    color: var(--muted);
  }
  .icon svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
  }
  .icon:hover,
  .icon.on {
    color: var(--teal);
    border-color: var(--teal);
  }

  .sight {
    margin: 0 -4px;
  }

  .readout {
    display: grid;
    grid-template-columns: 1fr 1.3fr 1fr;
    border-top: 1px solid var(--hair);
    border-bottom: 1px solid var(--hair);
  }
  .cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 7px 0;
  }
  .cell + .cell {
    border-left: 1px solid var(--hair);
    padding-left: 10px;
  }
  .v {
    font-size: 20px;
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
  }
  .v.big {
    font-size: 26px;
    line-height: 1.05;
    text-shadow: 0 0 16px color-mix(in srgb, var(--accent) 35%, transparent);
  }
  .v i {
    font-style: normal;
    font-size: 10px;
    margin-left: 3px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--accent);
  }
  .arcs {
    margin-left: 6px;
  }
  .arcs button {
    font-size: 9px;
    letter-spacing: 0.1em;
    padding: 0 4px;
    color: var(--muted);
    border: 1px solid var(--hair-strong);
    border-radius: 2px;
  }
  .arcs button.on {
    color: var(--bg);
    background: var(--teal);
    border-color: var(--teal);
  }

  .envelope {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .envelope :global(.bar) {
    flex: 1;
  }
  .status {
    font-size: 9px;
    letter-spacing: 0.1em;
    color: var(--alarm);
    white-space: nowrap;
    padding-bottom: 8px;
  }
  .status.ok {
    color: var(--teal);
  }

  .points {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: auto;
  }
  .swap {
    align-self: center;
    color: var(--muted);
    font-size: 13px;
    line-height: 1;
    padding: 0 8px;
  }
  .swap:hover {
    color: var(--teal);
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  kbd {
    font-family: var(--font-num);
    font-size: 11px;
    color: var(--teal);
    border: 1px solid var(--hair-strong);
    border-radius: 3px;
    padding: 0 4px;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--alarm);
  }

  .toast {
    position: fixed;
    left: 10px;
    right: 10px;
    bottom: 10px;
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px;
    text-align: left;
    background: var(--surface);
    border: 1px solid var(--alarm);
    border-radius: 4px;
    font-size: 12px;
    color: var(--muted);
    animation: rise 0.2s ease-out;
  }
  .toast img {
    width: 96px;
    border: 1px solid var(--hair);
  }
  .toast b {
    display: block;
    color: var(--alarm);
    font-weight: 600;
    letter-spacing: 0.04em;
  }
  @keyframes rise {
    from {
      transform: translateY(12px);
      opacity: 0;
    }
  }
</style>
