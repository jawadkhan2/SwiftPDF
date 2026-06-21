<script lang="ts">
  import "../app.css";
  import TitleBar from "$lib/components/TitleBar.svelte";
  let { children } = $props();
</script>

<div class="app-shell">
  <TitleBar />
  <div class="app-content">
    {@render children()}
  </div>
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
