<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { doc } from "$lib/stores/document.svelte";
  import {
    renderPage,
    toDataUrl,
    saveSignedPdf,
    editedName,
    type RenderResult,
    type Stamp,
  } from "$lib/api";
  import SignatureModal from "$lib/components/SignatureModal.svelte";

  type EditorStamp =
    | {
        id: string;
        page: number;
        type: "text";
        fx: number;
        fy: number;
        fontFrac: number;
        text: string;
        color: [number, number, number];
      }
    | {
        id: string;
        page: number;
        type: "image";
        fx: number;
        fy: number;
        fw: number;
        fh: number;
        aspect: number; // natural width / height of the image
        dataUrl: string;
      };

  const current = $derived(doc.current);

  let pageIndex = $state(0);
  let render = $state<RenderResult | null>(null);
  let loading = $state(false);
  let stamps = $state<EditorStamp[]>([]);
  let selectedId = $state<string | null>(null);
  let showSigModal = $state(false);
  let saving = $state(false);
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  let pageEl = $state<HTMLDivElement | null>(null);
  let pageW = $state(1);
  let pageH = $state(1);

  const colors: [number, number, number][] = [
    [17, 17, 17],
    [37, 99, 246],
    [206, 42, 42],
  ];

  let nextId = 0;
  const newId = () => `s${nextId++}`;
  const clamp = (v: number, lo: number, hi: number) =>
    Math.min(hi, Math.max(lo, v));

  function measure() {
    if (pageEl) {
      pageW = pageEl.clientWidth;
      pageH = pageEl.clientHeight;
    }
  }

  onMount(() => {
    if (!doc.current) {
      goto("/");
      return;
    }
    void loadPage(0);

    const ro = new ResizeObserver(measure);
    if (pageEl) ro.observe(pageEl);
    return () => ro.disconnect();
  });

  async function loadPage(i: number) {
    if (!current) return;
    loading = true;
    selectedId = null;
    try {
      render = await renderPage(current.doc_id, i, 1500);
      pageIndex = i;
      queueMicrotask(measure);
    } finally {
      loading = false;
    }
  }

  const visible = $derived(stamps.filter((s) => s.page === pageIndex));

  // --- Adding items ---------------------------------------------------------
  function addText() {
    const s: EditorStamp = {
      id: newId(),
      page: pageIndex,
      type: "text",
      fx: 0.12,
      fy: 0.12,
      fontFrac: 0.03,
      text: "Type here",
      color: colors[0],
    };
    stamps = [...stamps, s];
    selectedId = s.id;
  }

  function onSignaturePicked(dataUrl: string) {
    showSigModal = false;
    const img = new Image();
    img.onload = () => {
      const aspect = img.naturalWidth / Math.max(img.naturalHeight, 1);
      const fw = 0.28;
      // Preserve the image's aspect within the page box (uses display aspect).
      const fh = (fw * (pageW / pageH)) / aspect;
      const s: EditorStamp = {
        id: newId(),
        page: pageIndex,
        type: "image",
        fx: 0.12,
        fy: 0.6,
        fw,
        fh,
        aspect,
        dataUrl,
      };
      stamps = [...stamps, s];
      selectedId = s.id;
    };
    img.src = dataUrl;
  }

  function deleteStamp(id: string) {
    stamps = stamps.filter((s) => s.id !== id);
    if (selectedId === id) selectedId = null;
  }

  function setText(id: string, value: string) {
    const s = stamps.find((x) => x.id === id);
    if (s && s.type === "text") s.text = value;
  }

  function setColor(id: string, c: [number, number, number]) {
    const s = stamps.find((x) => x.id === id);
    if (s && s.type === "text") s.color = c;
  }

  function bumpSize(id: string, delta: number) {
    const s = stamps.find((x) => x.id === id);
    if (s && s.type === "text")
      s.fontFrac = clamp(s.fontFrac + delta, 0.012, 0.12);
  }

  // --- Drag to move ---------------------------------------------------------
  let drag: { id: string; dx: number; dy: number } | null = null;

  function startMove(e: PointerEvent, s: EditorStamp) {
    if (!pageEl) return;
    const r = pageEl.getBoundingClientRect();
    const px = (e.clientX - r.left) / r.width;
    const py = (e.clientY - r.top) / r.height;
    drag = { id: s.id, dx: px - s.fx, dy: py - s.fy };
    selectedId = s.id;
    (e.target as Element).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  // --- Resize (images) ------------------------------------------------------
  let resize: { id: string; startX: number; startFw: number } | null = null;

  function startResize(e: PointerEvent, s: EditorStamp) {
    if (s.type !== "image") return;
    resize = { id: s.id, startX: e.clientX, startFw: s.fw };
    (e.target as Element).setPointerCapture(e.pointerId);
    e.preventDefault();
    e.stopPropagation();
  }

  function onPagePointerMove(e: PointerEvent) {
    if (!pageEl) return;
    const r = pageEl.getBoundingClientRect();
    if (drag) {
      const px = (e.clientX - r.left) / r.width;
      const py = (e.clientY - r.top) / r.height;
      const s = stamps.find((x) => x.id === drag!.id);
      if (s) {
        s.fx = clamp(px - drag.dx, 0, 0.99);
        s.fy = clamp(py - drag.dy, 0, 0.99);
      }
    } else if (resize) {
      const s = stamps.find((x) => x.id === resize!.id);
      if (s && s.type === "image") {
        const dw = (e.clientX - resize.startX) / r.width;
        s.fw = clamp(resize.startFw + dw, 0.06, 1);
        s.fh = (s.fw * (pageW / pageH)) / s.aspect;
      }
    }
  }

  function endPointer() {
    drag = null;
    resize = null;
  }

  // --- Save -----------------------------------------------------------------
  async function save() {
    if (!current) return;
    const payload: Stamp[] = [];
    for (const s of stamps) {
      if (s.type === "text") {
        if (!s.text.trim()) continue;
        payload.push({
          kind: "Text",
          page: s.page,
          fx: s.fx,
          fy: s.fy,
          fh: s.fontFrac,
          text: s.text,
          color: s.color,
        });
      } else {
        payload.push({
          kind: "Image",
          page: s.page,
          fx: s.fx,
          fy: s.fy,
          fw: s.fw,
          fh: s.fh,
          png_base64: s.dataUrl.split(",")[1] ?? "",
        });
      }
    }
    if (payload.length === 0) {
      toast = { kind: "err", msg: "Add some text or a signature first." };
      return;
    }
    saving = true;
    toast = null;
    try {
      const path = await saveSignedPdf(
        current.doc_id,
        payload,
        editedName(current.source_name),
      );
      if (path) toast = { kind: "ok", msg: `Saved a copy to ${path}` };
    } catch (e) {
      toast = { kind: "err", msg: String(e) };
    } finally {
      saving = false;
    }
  }
</script>

<div class="screen">
  <header class="toolbar">
    <button class="btn btn-ghost" onclick={() => goto("/")}>← Home</button>
    <div class="title">
      <strong>Fill &amp; Sign</strong>
      {#if current}<span class="muted">{current.source_name}</span>{/if}
    </div>
    <div class="spacer"></div>
    <button class="btn" onclick={addText}>+ Text</button>
    <button class="btn" onclick={() => (showSigModal = true)}>✎ Signature</button>
    <button class="btn btn-primary" onclick={save} disabled={saving}>
      {saving ? "Saving…" : "Save a copy"}
    </button>
  </header>

  {#if toast}
    <div class="banner {toast.kind}" role="status">{toast.msg}</div>
  {/if}

  {#if current}
    <div class="stage">
      {#if current.page_count > 1}
        <div class="pager">
          <button
            class="btn btn-ghost"
            disabled={pageIndex === 0 || loading}
            onclick={() => loadPage(pageIndex - 1)}>‹ Prev</button
          >
          <span class="muted">Page {pageIndex + 1} of {current.page_count}</span>
          <button
            class="btn btn-ghost"
            disabled={pageIndex >= current.page_count - 1 || loading}
            onclick={() => loadPage(pageIndex + 1)}>Next ›</button
          >
        </div>
      {/if}

      <div class="page-wrap">
        {#if render}
          <div
            class="page"
            bind:this={pageEl}
            role="application"
            aria-label="Page editor — drag items to position them"
            style={`aspect-ratio: ${render.width_px} / ${render.height_px};`}
            onpointerdown={() => (selectedId = null)}
            onpointermove={onPagePointerMove}
            onpointerup={endPointer}
            onpointerleave={endPointer}
          >
            <img
              class="bg"
              src={toDataUrl(render)}
              alt={`Page ${pageIndex + 1}`}
              draggable="false"
            />

            {#each visible as s (s.id)}
              <div
                class="stamp"
                class:selected={selectedId === s.id}
                style={`left:${s.fx * 100}%; top:${s.fy * 100}%;` +
                  (s.type === "image"
                    ? `width:${s.fw * 100}%; height:${s.fh * 100}%;`
                    : "")}
                onpointerdown={(e) => {
                  e.stopPropagation();
                  selectedId = s.id;
                }}
                role="button"
                tabindex="0"
              >
                <span
                  class="handle"
                  role="button"
                  tabindex="-1"
                  aria-label="Move"
                  title="Drag to move"
                  onpointerdown={(e) => startMove(e, s)}>⠿</span
                >
                <button
                  class="del"
                  title="Remove"
                  onpointerdown={(e) => {
                    e.stopPropagation();
                    deleteStamp(s.id);
                  }}>✕</button
                >

                {#if s.type === "text"}
                  <div
                    class="text-body"
                    contenteditable="true"
                    style={`font-size:${s.fontFrac * pageH}px; color: rgb(${s.color[0]},${s.color[1]},${s.color[2]});`}
                    oninput={(e) =>
                      setText(s.id, (e.currentTarget as HTMLElement).innerText)}
                    onfocus={() => (selectedId = s.id)}
                  >
                    {s.text}
                  </div>

                  {#if selectedId === s.id}
                    <div class="text-tools" contenteditable="false">
                      <button
                        onpointerdown={(e) => {
                          e.stopPropagation();
                          bumpSize(s.id, -0.005);
                        }}>A−</button
                      >
                      <button
                        onpointerdown={(e) => {
                          e.stopPropagation();
                          bumpSize(s.id, 0.005);
                        }}>A+</button
                      >
                      {#each colors as c (c.join())}
                        <button
                          class="swatch"
                          style={`background: rgb(${c[0]},${c[1]},${c[2]})`}
                          aria-label="text colour"
                          onpointerdown={(e) => {
                            e.stopPropagation();
                            setColor(s.id, c);
                          }}
                        ></button>
                      {/each}
                    </div>
                  {/if}
                {:else}
                  <img class="sig" src={s.dataUrl} alt="signature" draggable="false" />
                  {#if selectedId === s.id}
                    <span
                      class="resize"
                      role="button"
                      tabindex="-1"
                      aria-label="Resize"
                      title="Drag to resize"
                      onpointerdown={(e) => startResize(e, s)}
                    ></span>
                  {/if}
                {/if}
              </div>
            {/each}
          </div>
        {:else if loading}
          <div class="placeholder muted">Loading page…</div>
        {/if}
      </div>

      <p class="hint muted">
        Add text or a signature, drag to position, then “Save a copy”. Your
        original file is never changed.
      </p>
    </div>
  {/if}
</div>

{#if showSigModal}
  <SignatureModal
    onpick={onSignaturePicked}
    onclose={() => (showSigModal = false)}
  />
{/if}

<style>
  .screen {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem 1.2rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .title {
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .spacer {
    flex: 1;
  }
  .stage {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 1rem;
    gap: 0.8rem;
  }
  .pager {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .page-wrap {
    width: 100%;
    display: flex;
    justify-content: center;
  }
  .page {
    position: relative;
    width: min(800px, 100%);
    background: #fff;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.16);
    user-select: none;
  }
  .bg {
    display: block;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .placeholder {
    padding: 4rem;
  }
  .stamp {
    position: absolute;
    min-width: 1.5rem;
    min-height: 1rem;
  }
  .stamp.selected {
    outline: 1.5px dashed var(--primary);
    outline-offset: 2px;
  }
  .handle {
    position: absolute;
    top: -1.4rem;
    left: 0;
    cursor: grab;
    background: var(--primary);
    color: #fff;
    border-radius: 4px;
    padding: 0 0.3rem;
    font-size: 0.8rem;
    opacity: 0;
    touch-action: none;
  }
  .del {
    position: absolute;
    top: -1.4rem;
    right: 0;
    border: none;
    background: var(--danger);
    color: #fff;
    border-radius: 4px;
    width: 1.3rem;
    height: 1.3rem;
    font-size: 0.75rem;
    cursor: pointer;
    opacity: 0;
    padding: 0;
  }
  .stamp:hover .handle,
  .stamp:hover .del,
  .stamp.selected .handle,
  .stamp.selected .del {
    opacity: 1;
  }
  .text-body {
    white-space: nowrap;
    outline: none;
    line-height: 1.1;
    cursor: text;
    padding: 1px 2px;
  }
  .text-tools {
    position: absolute;
    bottom: -2.2rem;
    left: 0;
    display: flex;
    gap: 0.25rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.2rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
  }
  .text-tools button {
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 4px;
    min-width: 1.6rem;
    height: 1.6rem;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .swatch {
    width: 1.6rem;
    padding: 0;
  }
  .sig {
    width: 100%;
    height: 100%;
    object-fit: fill;
    pointer-events: none;
  }
  .resize {
    position: absolute;
    right: -7px;
    bottom: -7px;
    width: 14px;
    height: 14px;
    background: var(--primary);
    border: 2px solid #fff;
    border-radius: 50%;
    cursor: nwse-resize;
    touch-action: none;
  }
  .hint {
    font-size: 0.85rem;
    text-align: center;
    max-width: 32rem;
  }
  .banner {
    margin: 0.8rem 1.2rem 0;
    padding: 0.7rem 1rem;
    border-radius: var(--radius-sm);
    align-self: stretch;
  }
  .banner.ok {
    background: #e6f4ec;
    color: var(--success);
  }
  .banner.err {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
