<script lang="ts" context="module">
  export const title = "Demo";
</script>

<script>
  import { createEventDispatcher } from 'svelte';

  export let count = 0;
  export const mode = 'demo';
  export function inc() { count += 1; }

  let items = ["a", "b"];
  let ready = false;
  let promise = Promise.resolve("ok");
  let html = "<strong>safe</strong>";
  const dispatch = createEventDispatcher();
  dispatch('submit');

  let { title: slotTitle } = $props();
</script>

<style>
  :global(body) { margin: 0; }
  main { padding: 1rem; --accent: #ff6600; }
</style>

<main id="main" on:click={inc} class:active={ready} bind:this={root} use:tooltip>
  <MyCard id="main" value={items.length} on:submit={inc} let:item />

  {#if ready}
    <p>{title}</p>
  {:else}
    <p>loading</p>
  {/if}

  {#each items as item, i}
    <li>{item} {i}</li>
  {/each}

  {#await promise}
    <p>waiting</p>
  {:then value}
    <p>{value}</p>
  {:catch err}
    <p>{err}</p>
  {/await}

  {#key items.length}
    <span>{items.length}</span>
  {/key}

  {#snippet row(name)}
    <div>{name}</div>
  {/snippet}

  {@const doubled = items.length * 2}
  {@debug doubled}
  {@html html}
  {@render row("x")}
  {@render missingRow("y")}

  <p>{doubled}</p>
</main>
