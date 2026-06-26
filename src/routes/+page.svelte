<script lang="ts">
  import { goto } from "$app/navigation";
  import { pickPdf, openPdf } from "$lib/api";
  import { setDoc } from "$lib/stores/document.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import { updater } from "$lib/updater.svelte";

  let busy = $state(false);
  let error = $state<string | null>(null);

  // Open a single PDF, register it, and navigate to the chosen tool screen.
  async function openAndGo(route: string) {
    error = null;
    const path = await pickPdf();
    if (!path) return; // user cancelled
    busy = true;
    try {
      const result = await openPdf(path);
      await setDoc(result);
      await goto(route);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const tiles = [
    {
      key: "organize",
      title: "Organize pages",
      desc: "Reorder, rotate, or delete pages",
      icon: "▤",
      action: () => openAndGo("/organize"),
    },
    {
      key: "sign",
      title: "Fill & Sign",
      desc: "Type into forms and add your signature",
      icon: "✎",
      action: () => openAndGo("/sign"),
    },
    {
      key: "merge",
      title: "Merge PDFs",
      desc: "Combine several files into one",
      icon: "⧉",
      action: () => goto("/merge"),
    },
    {
      key: "split",
      title: "Split / extract",
      desc: "Pull out pages into a new file",
      icon: "✂",
      action: () => openAndGo("/split"),
    },
  ];
</script>

<main class="home">
  <header class="hero">
    <div class="logo"><Logo size={64} /></div>
    <h1>SwiftPDF</h1>
    <p class="muted">
      Edit your PDFs right on your computer. Nothing is uploaded — your files
      stay private.
    </p>
  </header>

  {#if error}
    <div class="banner error" role="alert">{error}</div>
  {/if}

  <section class="tiles" aria-busy={busy}>
    {#each tiles as tile (tile.key)}
      <button class="tile" onclick={tile.action} disabled={busy}>
        <span class="tile-icon" aria-hidden="true">{tile.icon}</span>
        <span class="tile-title">{tile.title}</span>
        <span class="tile-desc">{tile.desc}</span>
      </button>
    {/each}
  </section>

  <p class="hint muted">Pick what you'd like to do — we'll guide you.</p>

  <button
    class="update-link"
    onclick={() => updater.check(true)}
    disabled={updater.busy}
  >
    {updater.phase === "checking" ? "Checking…" : "Check for updates"}
  </button>
</main>

<style>
  .home {
    max-width: 760px;
    margin: 0 auto;
    padding: 5vh 1.5rem 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .hero {
    text-align: center;
    margin-bottom: 2rem;
  }
  .logo {
    display: inline-flex;
    line-height: 0;
  }
  .hero h1 {
    font-size: 2.4rem;
    margin: 0.4rem 0 0.6rem;
  }
  .hero p {
    max-width: 30rem;
    margin: 0 auto;
    line-height: 1.5;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
    width: 100%;
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    gap: 0.35rem;
    padding: 1.4rem 1.4rem 1.5rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    transition: transform 0.08s, box-shadow 0.15s, border-color 0.15s;
  }
  .tile:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: var(--shadow-lg);
    border-color: var(--primary);
  }
  .tile:disabled {
    opacity: 0.6;
  }
  .tile-icon {
    font-size: 1.8rem;
    color: var(--primary);
  }
  .tile-title {
    font-size: 1.2rem;
    font-weight: 700;
  }
  .tile-desc {
    color: var(--text-soft);
    font-size: 0.95rem;
  }
  .hint {
    margin-top: 1.5rem;
  }
  .update-link {
    margin-top: 0.6rem;
    background: none;
    border: none;
    color: var(--text-soft);
    font-size: 0.85rem;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .update-link:hover:not(:disabled) {
    color: var(--primary);
  }
  .update-link:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .banner {
    width: 100%;
    padding: 0.8rem 1rem;
    border-radius: var(--radius-sm);
    margin-bottom: 1rem;
  }
  .banner.error {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
