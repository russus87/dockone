<script lang="ts">
  import { api, ago } from "./api";
  import type { Container, Settings } from "./types";

  let {
    hostId,
    query = "",
    settings,
    onSettings,
  }: {
    hostId: string;
    query?: string;
    settings: Settings;
    onSettings: (s: Settings) => void;
  } = $props();

  let items = $state<Container[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let busy = $state<Record<string, boolean>>({});

  let logsFor = $state<Container | null>(null);
  let logText = $state("");
  let inspectFor = $state<Container | null>(null);
  let inspectText = $state("");

  async function load() {
    loading = true;
    error = null;
    try {
      items = await api.containers(hostId);
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

  function favKey(c: Container) {
    return `${hostId}/${c.name}`;
  }
  function isFav(c: Container) {
    return settings.favorite_containers.includes(favKey(c));
  }
  async function toggleFav(c: Container) {
    onSettings(await api.toggleFavoriteContainer(favKey(c)));
  }

  async function act(c: Container, action: string) {
    busy = { ...busy, [c.id]: true };
    try {
      await api.containerAction(hostId, c.id, action);
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = { ...busy, [c.id]: false };
    }
  }

  async function openLogs(c: Container) {
    logsFor = c;
    logText = "Caricamento…";
    try {
      logText = (await api.containerLogs(hostId, c.id)) || "(nessun log)";
    } catch (e) {
      logText = String(e);
    }
  }

  async function openInspect(c: Container) {
    inspectFor = c;
    inspectText = "Caricamento…";
    try {
      inspectText = JSON.stringify(await api.inspectContainer(hostId, c.id), null, 2);
    } catch (e) {
      inspectText = String(e);
    }
  }

  const filtered = $derived(
    items
      .filter((c) => {
        const q = query.trim().toLowerCase();
        if (!q) return true;
        return (
          c.name.toLowerCase().includes(q) ||
          c.image.toLowerCase().includes(q)
        );
      })
      .sort((a, b) => {
        const fa = isFav(a) ? 0 : 1;
        const fb = isFav(b) ? 0 : 1;
        if (fa !== fb) return fa - fb;
        const ra = a.state === "running" ? 0 : 1;
        const rb = b.state === "running" ? 0 : 1;
        if (ra !== rb) return ra - rb;
        return a.name.localeCompare(b.name);
      }),
  );
</script>

<div>
  {#if error}<div class="err-banner">{error}</div>{/if}

  {#if loading}
    <div class="loading">Caricamento container…</div>
  {:else if filtered.length === 0}
    <div class="empty">Nessun container.</div>
  {:else}
    <table class="table">
      <thead>
        <tr>
          <th style="width:28px"></th>
          <th>Nome</th>
          <th>Immagine</th>
          <th>Stato</th>
          <th>Porte</th>
          <th>Creato</th>
          <th style="text-align:right">Azioni</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as c (c.id)}
          <tr>
            <td>
              <button class="star {isFav(c) ? 'on' : ''}" title="Preferito" onclick={() => toggleFav(c)}>
                {isFav(c) ? "★" : "☆"}
              </button>
            </td>
            <td class="row-title">{c.name}</td>
            <td class="mono">{c.image}</td>
            <td>
              <span class="badge {c.state}"><span class="dot"></span>{c.state || "?"}</span>
            </td>
            <td class="mono">{c.ports.join(", ") || "—"}</td>
            <td class="mono">{ago(c.created)}</td>
            <td>
              <div class="actions" style="justify-content:flex-end">
                {#if c.state === "running"}
                  <button class="act danger" disabled={busy[c.id]} onclick={() => act(c, "stop")}>Stop</button>
                  <button class="act" disabled={busy[c.id]} onclick={() => act(c, "restart")}>Restart</button>
                {:else}
                  <button class="act primary" disabled={busy[c.id]} onclick={() => act(c, "start")}>Start</button>
                {/if}
                <button class="act" onclick={() => openLogs(c)}>Logs</button>
                <button class="act" onclick={() => openInspect(c)}>Inspect</button>
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if logsFor}
  <div class="overlay" onclick={() => (logsFor = null)}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <div class="modal-head">
        <b>Logs · {logsFor.name}</b>
        <button class="icon-btn" onclick={() => (logsFor = null)}>✕</button>
      </div>
      <div class="modal-body"><pre class="logs">{logText}</pre></div>
    </div>
  </div>
{/if}

{#if inspectFor}
  <div class="overlay" onclick={() => (inspectFor = null)}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <div class="modal-head">
        <b>Inspect · {inspectFor.name}</b>
        <button class="icon-btn" onclick={() => (inspectFor = null)}>✕</button>
      </div>
      <div class="modal-body"><pre class="json">{inspectText}</pre></div>
    </div>
  </div>
{/if}
