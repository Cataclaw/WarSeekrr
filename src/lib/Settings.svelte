<script lang="ts">
  import { untrack } from "svelte";
  import type { CaptureReport, Settings } from "./api";

  let {
    settings,
    lastCapture,
    error = "",
    onsave,
    onclose,
  }: {
    settings: Settings;
    lastCapture: CaptureReport | null;
    error?: string;
    onsave: (s: Settings) => void;
    onclose: () => void;
  } = $props();

  let draft = $state(untrack(() => structuredClone($state.snapshot(settings))));
</script>

<div class="scrim" role="presentation" onclick={onclose}></div>
<section class="sheet" aria-label="Settings">
  <header>
    <span class="label">Settings</span>
    <button class="x" onclick={onclose} aria-label="Close">×</button>
  </header>

  <div class="group">
    <span class="label">Hotkeys</span>
    <label><span>Mortar position</span><input class="num" bind:value={draft.hotkeyArtillery} /></label>
    <label><span>Target</span><input class="num" bind:value={draft.hotkeyTarget} /></label>
    <p class="hint">F1–F12, or combos like Alt+Q, Ctrl+Shift+1</p>
  </div>

  <div class="group">
    <span class="label">Capture box · px at 1440p</span>
    <div class="grid">
      <label><span>Left</span><input class="num" type="number" bind:value={draft.region.left} /></label>
      <label><span>Right</span><input class="num" type="number" bind:value={draft.region.right} /></label>
      <label><span>Up</span><input class="num" type="number" bind:value={draft.region.up} /></label>
      <label><span>Down</span><input class="num" type="number" bind:value={draft.region.down} /></label>
      <label><span>Threshold</span><input class="num" type="number" min="0" max="255" bind:value={draft.region.threshold} /></label>
      <label><span>Upscale</span><input class="num" type="number" min="1" max="6" bind:value={draft.region.upscale} /></label>
    </div>
  </div>

  {#if lastCapture}
    <div class="group">
      <span class="label">Last capture · {lastCapture.kind}</span>
      {#if lastCapture.image}<img src={lastCapture.image} alt="Captured region" />{/if}
      <pre class="num">{lastCapture.text || "(no text)"}</pre>
    </div>
  {/if}

  {#if error}<p class="error">{error}</p>{/if}

  <footer>
    <button class="ghost" onclick={onclose}>Cancel</button>
    <button class="primary" onclick={() => onsave(draft)}>Save</button>
  </footer>
</section>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(2 8 10 / 0.7);
    backdrop-filter: blur(2px);
  }
  .sheet {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    max-height: 88vh;
    overflow-y: auto;
    background: var(--surface);
    border-top: 1px solid var(--hair-strong);
    padding: 12px 14px 14px;
    animation: rise 0.22s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  @keyframes rise {
    from {
      transform: translateY(24px);
      opacity: 0;
    }
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }
  .x {
    font-size: 18px;
    color: var(--muted);
  }
  .group {
    padding: 10px 0;
    border-bottom: 1px solid var(--hair);
  }
  .group > .label {
    display: block;
    margin-bottom: 6px;
    color: var(--teal);
  }
  label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding: 3px 0;
    color: var(--text);
  }
  input {
    all: unset;
    width: 96px;
    padding: 4px 6px;
    background: var(--bg);
    border: 1px solid var(--hair);
    border-radius: 3px;
    color: var(--text);
    font-family: var(--font-num);
    user-select: text;
  }
  input:focus {
    border-color: var(--teal);
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    column-gap: 16px;
  }
  .grid input {
    width: 52px;
  }
  .hint {
    margin: 4px 0 0;
    font-size: 11px;
    color: var(--muted);
  }
  img {
    display: block;
    max-width: 100%;
    border: 1px solid var(--hair);
    image-rendering: pixelated;
  }
  pre {
    margin: 6px 0 0;
    font-size: 11px;
    color: var(--muted);
    white-space: pre-wrap;
    user-select: text;
  }
  .error {
    color: var(--alarm);
    font-size: 12px;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 12px;
  }
  .ghost,
  .primary {
    padding: 6px 16px;
    border-radius: 3px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: 11px;
  }
  .ghost {
    border: 1px solid var(--hair-strong);
    color: var(--muted);
  }
  .primary {
    background: var(--teal);
    color: var(--bg);
    font-weight: 700;
  }
</style>
