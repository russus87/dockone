<script lang="ts">
  import { api } from "./api";
  import type { Volume } from "./types";

  let { hostId, query = "" }: { hostId: string; query?: string } = $props();

  let items = $state<Volume[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      items = await api.volumes(hostId);
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
    items.filter((v) =>
      v.name.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );
</script>

<div>
  {#if error}<div class="err-banner">{error}</div>{/if}
  {#if loading}
    <div class="loading">Caricamento volumi…</div>
  {:else if filtered.length === 0}
    <div class="empty">Nessun volume.</div>
  {:else}
    <table class="table">
      <thead>
        <tr>
          <th>Nome</th>
          <th>Driver</th>
          <th>Mountpoint</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as v (v.name)}
          <tr>
            <td class="row-title">{v.name}</td>
            <td class="mono">{v.driver}</td>
            <td class="mono">{v.mountpoint}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
