<script lang="ts">
  import { api, humanBytes, ago } from "./api";
  import type { Image } from "./types";

  let { hostId, query = "" }: { hostId: string; query?: string } = $props();

  let items = $state<Image[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      items = await api.images(hostId);
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
    items.filter((i) => {
      const q = query.trim().toLowerCase();
      if (!q) return true;
      return i.tags.some((t) => t.toLowerCase().includes(q)) || i.id.includes(q);
    }),
  );
</script>

<div>
  {#if error}<div class="err-banner">{error}</div>{/if}
  {#if loading}
    <div class="loading">Caricamento immagini…</div>
  {:else if filtered.length === 0}
    <div class="empty">Nessuna immagine.</div>
  {:else}
    <table class="table">
      <thead>
        <tr>
          <th>Repository / Tag</th>
          <th>Image ID</th>
          <th>Dimensione</th>
          <th>Creata</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as i (i.id)}
          <tr>
            <td>
              {#if i.tags.length}
                {#each i.tags as t}<span class="tag">{t}</span>{/each}
              {:else}
                <span class="mono">&lt;none&gt;</span>
              {/if}
            </td>
            <td class="mono">{i.id}</td>
            <td class="mono">{humanBytes(i.size)}</td>
            <td class="mono">{ago(i.created)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
