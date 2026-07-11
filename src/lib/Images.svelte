<script lang="ts">
  import { api, humanBytes, ago } from "./api";
  import type { Image } from "./types";

  let { hostId, query = "" }: { hostId: string; query?: string } = $props();

  let items = $state<Image[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let pullName = $state("");
  let pulling = $state(false);
  let busy = $state<Record<string, boolean>>({});

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

  async function pull() {
    const name = pullName.trim();
    if (!name) return;
    pulling = true;
    error = null;
    try {
      await api.pullImage(hostId, name);
      pullName = "";
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      pulling = false;
    }
  }

  async function remove(i: Image) {
    if (!confirm(`Rimuovere l'immagine ${i.tags[0] ?? i.id}?`)) return;
    busy = { ...busy, [i.id]: true };
    try {
      await api.removeImage(hostId, i.tags[0] ?? i.id);
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = { ...busy, [i.id]: false };
    }
  }

  const filtered = $derived(
    items.filter((i) => {
      const q = query.trim().toLowerCase();
      if (!q) return true;
      return i.tags.some((t) => t.toLowerCase().includes(q)) || i.id.includes(q);
    }),
  );
</script>

<div>
  <div class="host-form" style="margin-bottom:16px">
    <input placeholder="Pull immagine — es. nginx:latest, ghcr.io/owner/app:tag" bind:value={pullName} onkeydown={(e) => e.key === "Enter" && pull()} />
    <button class="btn" disabled={pulling} onclick={pull}>{pulling ? "Download…" : "Pull"}</button>
  </div>

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
          <th style="text-align:right">Azioni</th>
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
            <td style="text-align:right">
              <button class="act danger" disabled={busy[i.id]} onclick={() => remove(i)}>Rimuovi</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
