<script lang="ts">
  import PocRunner from '$lib/components/PoCRunner.svelte';
  import CsvInspector from '$lib/components/CsvInspector.svelte';
</script>

<main>
  <h1>fairmorph-workbench</h1>
  <CsvInspector />
  <PocRunner />
</main>

<style>
  main {
    padding: 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
</style>