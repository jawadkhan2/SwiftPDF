<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  import { updater } from "$lib/updater.svelte";
  let { children } = $props();

  // Silent background check shortly after launch, so a banner appears without
  // blocking first paint. Failures (e.g. running in a plain browser) stay quiet.
  onMount(() => {
    const t = setTimeout(() => void updater.check(false), 3000);
    return () => clearTimeout(t);
  });
</script>

<div class="app-shell">
  <TitleBar />
  <div class="app-content">
    {@render children()}
  </div>
  <UpdateBanner />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }
  .app-content {
    flex: 1 1 auto;
    overflow-y: auto;
  }
</style>
