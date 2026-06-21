<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import Thumbnail from "$lib/components/Thumbnail.svelte";
  import { doc } from "$lib/stores/document.svelte";
  import { splitPdf } from "$lib/api";
  import { parseRanges, eachPageSeparately } from "$lib/ranges";

  const current = $derived(doc.current);

  let rangeText = $state("");
  let busy = $state(false);
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  onMount(() => {
    if (!doc.current) {
      goto("/");
      return;
    }
    // Sensible default: extract all pages as one file (user edits from here).
    rangeText = `1-${doc.current.page_count}`;
  });

  // Live-parsed plan from the text box.
  const parsed = $derived(
    current ? parseRanges(rangeText, current.page_count) : { groups: [], error: null },
  );

  const baseName = $derived(
    current ? current.source_name.replace(/\.pdf$/i, "") : "document",
  );

  function usePreset(groups: number[][]) {
    // Reflect a preset back into the text box so the user sees what it does.
    rangeText = groups
      .map((g) =>
        g.length === 1 ? `${g[0] + 1}` : `${g[0] + 1}-${g[g.length - 1] + 1}`,
      )
      .join(", ");
  }

  function describe(group: number[]): string {
    if (group.length === 1) return `Page ${group[0] + 1}`;
    return `Pages ${group[0] + 1}–${group[group.length - 1] + 1}`;
  }

  async function doSplit() {
    if (!current || parsed.error || parsed.groups.length === 0) return;
    busy = true;
    toast = null;
    try {
      const saved = await splitPdf(current.doc_id, parsed.groups, baseName);
      if (saved.length > 0) {
        const folder = saved[0].replace(/[\\/][^\\/]+$/, "");
        toast = {
          kind: "ok",
          msg: `Saved ${saved.length} file${saved.length > 1 ? "s" : ""} to ${folder}`,
        };
      }
    } catch (e) {
      toast = { kind: "err", msg: String(e) };
    } finally {
      busy = false;
    }
  }
</script>

<div class="screen">
  <header class="toolbar">
    <button class="btn btn-ghost" onclick={() => goto("/")}>← Home</button>
    <div class="title">
      <strong>Split / extract</strong>
      {#if current}
        <span class="muted">{current.source_name} · {current.page_count} pages</span>
      {/if}
    </div>
  </header>

  {#if toast}
    <div class="banner {toast.kind}" role="status">{toast.msg}</div>
  {/if}

  {#if current}
    <div class="layout">
      <aside class="panel">
        <h3>Which pages?</h3>
        <p class="muted small">
          Enter pages or ranges, separated by commas. Each one becomes its own
          file. For example <code>1-3, 4-6</code> makes two files.
        </p>
        <label class="sr-only" for="ranges">Page ranges</label>
        <input
          id="ranges"
          class="range-input"
          bind:value={rangeText}
          placeholder="e.g. 1-3, 4, 5-8"
        />

        <div class="presets">
          <button class="btn small" onclick={() => usePreset([[...Array(current.page_count).keys()]])}>
            All in one file
          </button>
          <button
            class="btn small"
            onclick={() => usePreset(eachPageSeparately(current.page_count))}
          >
            Every page separately
          </button>
        </div>

        {#if parsed.error}
          <p class="err small">{parsed.error}</p>
        {:else}
          <div class="preview">
            <strong>{parsed.groups.length}</strong> file{parsed.groups.length === 1
              ? ""
              : "s"} will be created:
            <ul>
              {#each parsed.groups as g, i (i)}
                <li>
                  <span class="file-name">{baseName}{parsed.groups.length > 1
                      ? ` (${i + 1})`
                      : ""}.pdf</span>
                  <span class="muted">— {describe(g)}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}

        <button
          class="btn btn-primary full"
          onclick={doSplit}
          disabled={busy || !!parsed.error || parsed.groups.length === 0}
        >
          {busy ? "Saving…" : "Save files…"}
        </button>
        <p class="muted small">Your original file is never changed.</p>
      </aside>

      <section class="grid">
        {#each current.pages as _p, i (i)}
          <figure class="card">
            <Thumbnail docId={current.doc_id} page={i} size={180} />
            <figcaption>Page {i + 1}</figcaption>
          </figure>
        {/each}
      </section>
    </div>
  {/if}
</div>

<style>
  .screen {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.7rem 1.2rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .title {
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .layout {
    flex: 1;
    display: grid;
    grid-template-columns: 320px 1fr;
    min-height: 0;
  }
  .panel {
    border-right: 1px solid var(--border);
    background: var(--surface);
    padding: 1.2rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  .panel h3 {
    margin: 0;
  }
  .small {
    font-size: 0.85rem;
  }
  code {
    background: var(--surface-2);
    padding: 0.05rem 0.3rem;
    border-radius: 4px;
  }
  .range-input {
    width: 100%;
    padding: 0.7rem 0.8rem;
    font-size: 1rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }
  .range-input:focus {
    outline: none;
    border-color: var(--primary);
  }
  .presets {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .btn.small {
    padding: 0.45rem 0.7rem;
    font-size: 0.85rem;
  }
  .preview {
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    padding: 0.7rem 0.9rem;
    font-size: 0.9rem;
  }
  .preview ul {
    margin: 0.4rem 0 0;
    padding-left: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .file-name {
    font-weight: 600;
  }
  .err {
    color: var(--danger);
  }
  .full {
    width: 100%;
  }
  .grid {
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 1rem;
    padding: 1.2rem;
    align-content: start;
  }
  .card {
    margin: 0;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  figcaption {
    text-align: center;
    font-size: 0.82rem;
    color: var(--text-soft);
  }
  .banner {
    margin: 0.8rem 1.2rem 0;
    padding: 0.7rem 1rem;
    border-radius: var(--radius-sm);
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
