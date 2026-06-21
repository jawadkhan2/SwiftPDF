<script lang="ts">
  // Renders one page to a thumbnail, but only once it's scrolled near the
  // viewport — so a thousands-of-page grid renders just what's on screen.
  import { getThumbnail } from "$lib/thumbnails";

  let {
    docId,
    page,
    size = 240,
    rotate = 0,
  }: { docId: string; page: number; size?: number; rotate?: number } = $props();

  let el = $state<HTMLDivElement | null>(null);
  let visible = $state(false);
  let src = $state<string | null>(null);
  let failed = $state(false);

  // Track viewport visibility. `rootMargin` warms thumbnails just before they
  // scroll in so they're usually ready by the time they're seen.
  $effect(() => {
    if (!el) return;
    const io = new IntersectionObserver(
      (entries) => {
        visible = entries[0]?.isIntersecting ?? false;
      },
      { rootMargin: "600px 0px" },
    );
    io.observe(el);
    return () => io.disconnect();
  });

  // Fetch (via the shared cache + render queue) whenever the target becomes
  // visible or changes. Drop the image when off-screen to bound memory — the
  // cache makes returning to it effectively instant.
  $effect(() => {
    // Establish reactive dependencies.
    const _docId = docId;
    const _page = page;
    const _size = size;

    if (!visible) {
      src = null;
      return;
    }

    failed = false;
    const ctrl = new AbortController();
    getThumbnail(_docId, _page, _size, ctrl.signal)
      .then((url) => {
        src = url;
      })
      .catch((e) => {
        if ((e as { name?: string })?.name !== "AbortError") failed = true;
      });
    return () => ctrl.abort();
  });
</script>

<div class="thumb" bind:this={el}>
  {#if src}
    <img
      {src}
      alt={`Page ${page + 1}`}
      draggable="false"
      style={`transform: rotate(${rotate}deg)`}
    />
  {:else if failed}
    <div class="ph error">!</div>
  {:else}
    <div class="ph">…</div>
  {/if}
</div>

<style>
  .thumb {
    width: 100%;
    aspect-ratio: 3 / 4;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface);
    border-radius: 6px;
    overflow: hidden;
  }
  .thumb img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
    transition: transform 0.2s ease;
  }
  .ph {
    color: var(--text-soft);
    font-size: 1.4rem;
  }
  .ph.error {
    color: var(--danger);
  }
</style>
