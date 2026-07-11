<script lang="ts">
  import { api } from "./api";
  import type { Volume } from "./types";

  let { hostId, query = "" }: { hostId: string; query?: string } = $props();

  let items = $state<Volume[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let busy = $state<Record<string, boolean>>({});

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

  async function remove(name: string) {
    if (!confirm(`Rimuovere il volume ${name}?`)) return;
    busy = { ...busy, [name]: true };
    try {
      await api.removeVolume(hostId, name);
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = { ...busy, [name]: false };
    }
  }

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
          <th style="text-align:right">Azioni</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as v (v.name)}
          <tr>
            <td class="row-title">{v.name}</td>
            <td class="mono">{v.driver}</td>
            <td class="mono">{v.mountpoint}</td>
            <td style="text-align:right">
              <button class="act danger" disabled={busy[v.name]} onclick={() => remove(v.name)}>Rimuovi</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
