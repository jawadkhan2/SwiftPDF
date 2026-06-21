<script lang="ts">
  import {
    signatures,
    addSignature,
    removeSignature,
  } from "$lib/stores/signatures.svelte";

  interface Props {
    onpick: (dataUrl: string) => void;
    onclose: () => void;
  }
  let { onpick, onclose }: Props = $props();

  type Tab = "draw" | "type" | "saved";
  let tab = $state<Tab>("draw");

  // --- Draw tab -------------------------------------------------------------
  let canvas = $state<HTMLCanvasElement | null>(null);
  let drawing = false;
  let hasInk = $state(false);

  function ctx() {
    return canvas?.getContext("2d") ?? null;
  }

  function pointerPos(e: PointerEvent) {
    const rect = canvas!.getBoundingClientRect();
    // Account for the canvas's internal pixel resolution vs CSS size.
    return {
      x: ((e.clientX - rect.left) / rect.width) * canvas!.width,
      y: ((e.clientY - rect.top) / rect.height) * canvas!.height,
    };
  }

  function startDraw(e: PointerEvent) {
    const c = ctx();
    if (!c) return;
    drawing = true;
    hasInk = true;
    const { x, y } = pointerPos(e);
    c.beginPath();
    c.moveTo(x, y);
    canvas!.setPointerCapture(e.pointerId);
  }

  function moveDraw(e: PointerEvent) {
    if (!drawing) return;
    const c = ctx();
    if (!c) return;
    const { x, y } = pointerPos(e);
    c.lineTo(x, y);
    c.strokeStyle = "#111";
    c.lineWidth = 3.5;
    c.lineCap = "round";
    c.lineJoin = "round";
    c.stroke();
  }

  function endDraw() {
    drawing = false;
  }

  function clearCanvas() {
    const c = ctx();
    if (c && canvas) c.clearRect(0, 0, canvas.width, canvas.height);
    hasInk = false;
  }

  // --- Type tab -------------------------------------------------------------
  const fonts = [
    { label: "Signature", css: "'Segoe Script', 'Brush Script MT', cursive" },
    { label: "Classic", css: "'Brush Script MT', 'Snell Roundhand', cursive" },
    { label: "Casual", css: "'Comic Sans MS', 'Segoe Print', cursive" },
  ];
  let typed = $state("");
  let fontIdx = $state(0);

  /** Render the typed name to a transparent PNG and return its data URL. */
  function typedToDataUrl(): string {
    const pad = 20;
    const fontPx = 80;
    const c = document.createElement("canvas");
    const m = c.getContext("2d")!;
    m.font = `${fontPx}px ${fonts[fontIdx].css}`;
    const w = Math.max(m.measureText(typed).width + pad * 2, 1);
    c.width = w;
    c.height = fontPx + pad * 2;
    const g = c.getContext("2d")!;
    g.font = `${fontPx}px ${fonts[fontIdx].css}`;
    g.fillStyle = "#111";
    g.textBaseline = "middle";
    g.fillText(typed, pad, c.height / 2);
    return c.toDataURL("image/png");
  }

  // --- Actions --------------------------------------------------------------
  let saveLabel = $state("");

  function currentDataUrl(): string | null {
    if (tab === "draw") {
      if (!hasInk || !canvas) return null;
      return canvas.toDataURL("image/png");
    }
    if (tab === "type") {
      if (!typed.trim()) return null;
      return typedToDataUrl();
    }
    return null;
  }

  function use() {
    const url = currentDataUrl();
    if (url) onpick(url);
  }

  function saveForReuse() {
    const url = currentDataUrl();
    if (!url) return;
    addSignature(saveLabel || (tab === "type" ? typed : "Signature"), url);
    saveLabel = "";
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div
  class="backdrop"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onclose();
  }}
>
  <div class="modal" role="dialog" aria-modal="true" aria-label="Add a signature" tabindex="-1">
    <header>
      <strong>Add your signature</strong>
      <button class="x" onclick={onclose} aria-label="Close">✕</button>
    </header>

    <nav class="tabs">
      <button class:active={tab === "draw"} onclick={() => (tab = "draw")}>Draw</button>
      <button class:active={tab === "type"} onclick={() => (tab = "type")}>Type</button>
      <button class:active={tab === "saved"} onclick={() => (tab = "saved")}>
        Saved {signatures.list.length ? `(${signatures.list.length})` : ""}
      </button>
    </nav>

    <div class="content">
      {#if tab === "draw"}
        <p class="muted small">Draw with your mouse or trackpad.</p>
        <canvas
          bind:this={canvas}
          width="500"
          height="200"
          class="pad"
          onpointerdown={startDraw}
          onpointermove={moveDraw}
          onpointerup={endDraw}
          onpointerleave={endDraw}
        ></canvas>
        <div class="row">
          <button class="btn btn-ghost" onclick={clearCanvas}>Clear</button>
        </div>
      {:else if tab === "type"}
        <p class="muted small">Type your name, then pick a style.</p>
        <input class="text-in" bind:value={typed} placeholder="Your name" />
        <div class="font-choices">
          {#each fonts as f, i (f.label)}
            <button
              class="font-choice"
              class:active={fontIdx === i}
              style={`font-family: ${f.css}`}
              onclick={() => (fontIdx = i)}
            >
              {typed.trim() || "Your name"}
            </button>
          {/each}
        </div>
      {:else}
        {#if signatures.list.length === 0}
          <p class="muted">No saved signatures yet. Create one in Draw or Type, then "Save for reuse".</p>
        {:else}
          <ul class="saved">
            {#each signatures.list as s (s.id)}
              <li>
                <img src={s.dataUrl} alt={s.label} />
                <span class="label">{s.label}</span>
                <button class="btn btn-primary small" onclick={() => onpick(s.dataUrl)}>Use</button>
                <button class="btn btn-ghost small" onclick={() => removeSignature(s.id)}>Delete</button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>

    {#if tab !== "saved"}
      <footer>
        <input class="label-in" bind:value={saveLabel} placeholder="Label (optional)" />
        <button class="btn btn-ghost" onclick={saveForReuse}>Save for reuse</button>
        <div class="spacer"></div>
        <button class="btn btn-primary" onclick={use}>Place on page</button>
      </footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(20, 28, 45, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }
  .modal {
    background: var(--surface);
    border-radius: var(--radius);
    width: min(560px, 92vw);
    max-height: 90vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.2rem;
    border-bottom: 1px solid var(--border);
  }
  .x {
    border: none;
    background: none;
    font-size: 1.1rem;
    cursor: pointer;
    color: var(--text-soft);
  }
  .tabs {
    display: flex;
    gap: 0.3rem;
    padding: 0.6rem 1.2rem 0;
  }
  .tabs button {
    border: none;
    background: none;
    padding: 0.5rem 0.9rem;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    cursor: pointer;
    font-size: 0.95rem;
    color: var(--text-soft);
  }
  .tabs button.active {
    background: var(--surface-2);
    color: var(--text);
    font-weight: 600;
  }
  .content {
    padding: 1rem 1.2rem;
    overflow-y: auto;
  }
  .small {
    font-size: 0.85rem;
  }
  .pad {
    width: 100%;
    height: auto;
    aspect-ratio: 5 / 2;
    background: #fff;
    border: 2px dashed var(--border);
    border-radius: var(--radius-sm);
    touch-action: none;
    cursor: crosshair;
  }
  .row {
    margin-top: 0.6rem;
    display: flex;
    gap: 0.5rem;
  }
  .text-in,
  .label-in {
    width: 100%;
    padding: 0.6rem 0.7rem;
    font-size: 1rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }
  .font-choices {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.8rem;
  }
  .font-choice {
    padding: 0.6rem 1rem;
    font-size: 1.7rem;
    border: 2px solid var(--border);
    border-radius: var(--radius-sm);
    background: #fff;
    cursor: pointer;
    text-align: left;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .font-choice.active {
    border-color: var(--primary);
  }
  .saved {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .saved li {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .saved img {
    height: 42px;
    max-width: 160px;
    object-fit: contain;
    background: #fff;
    border-radius: 4px;
  }
  .saved .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btn.small {
    padding: 0.35rem 0.6rem;
    font-size: 0.85rem;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.9rem 1.2rem;
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }
  .label-in {
    max-width: 12rem;
  }
  .spacer {
    flex: 1;
  }
</style>
