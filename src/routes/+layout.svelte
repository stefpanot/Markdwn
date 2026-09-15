<script lang="ts">
  // Polices vendorisées : une app de bureau ne doit pas dépendre du réseau
  // pour s'afficher correctement.
  import "@fontsource-variable/inter";
  import "@fontsource-variable/jetbrains-mono";
  import "@fontsource-variable/literata";
  import "$lib/tokens.css";
  import { app } from "$lib/state.svelte";

  let { children } = $props();

  $effect(() => {
    document.documentElement.dataset.theme = app.theme;
  });
</script>

{@render children()}
