<script lang="ts">
  import { api } from "./api";
  import type { Network } from "./types";

  let { hostId, query = "" }: { hostId: string; query?: string } = $props();

  let items = $state<Network[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      items = await api.networks(hostId);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    hostId;
    load();
  });

  const filtered = $derived(
    items.filter((n) =>
      n.name.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );
</script>

<div>
  {#if error}<div class="err-banner">{error}</div>{/if}
  {#if loading}
    <div class="loading">Caricamento reti…</div>
  {:else if filtered.length === 0}
    <div class="empty">Nessuna rete.</div>
  {:else}
    <table class="table">
      <thead>
        <tr>
          <th>Nome</th>
          <th>Driver</th>
          <th>Scope</th>
          <th>Network ID</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as n (n.id)}
          <tr>
            <td class="row-title">{n.name}</td>
            <td class="mono">{n.driver}</td>
            <td class="mono">{n.scope}</td>
            <td class="mono">{n.id}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
