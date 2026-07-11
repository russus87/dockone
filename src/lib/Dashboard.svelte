<script lang="ts">
  import { api, humanBytes } from "./api";
  import type { HostSummary } from "./types";

  let { query = "" }: { query?: string } = $props();

  let hosts = $state<HostSummary[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      hosts = await api.dashboard();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    load();
  });

  const filtered = $derived(
    hosts.filter((h) => h.name.toLowerCase().includes(query.trim().toLowerCase())),
  );

  const totals = $derived({
    running: hosts.reduce((a, h) => a + h.running, 0),
    containers: hosts.reduce((a, h) => a + h.containers, 0),
    images: hosts.reduce((a, h) => a + h.images, 0),
    online: hosts.filter((h) => h.online).length,
  });

  // highlight the busiest online host as a filled "featured" card
  const featuredId = $derived(
    hosts
      .filter((h) => h.online)
      .sort((a, b) => b.running - a.running)[0]?.id ?? null,
  );
</script>

<div>
  {#if error}<div class="err-banner">{error}</div>{/if}

  {#if loading}
    <div class="loading">Connessione agli host…</div>
  {:else}
    <div class="kpi-row">
      <div class="kpi"><b>{totals.online}/{hosts.length}</b><small>Host online</small></div>
      <div class="kpi"><b style="color:var(--green)">{totals.running}</b><small>In esecuzione</small></div>
      <div class="kpi"><b>{totals.containers}</b><small>Container totali</small></div>
      <div class="kpi"><b>{totals.images}</b><small>Immagini</small></div>
    </div>

    <p class="section-label">I tuoi host</p>
    <div class="grid">
      {#each filtered as h (h.id)}
        <div class="host-card {h.id === featuredId ? 'filled' : ''}">
          <div class="hc-top">
            <div class="hc-icon">🖳</div>
            <div class="hc-name">{h.name}</div>
            <span class="hc-badge badge {h.online ? 'running' : 'offline'}">
              <span class="dot"></span>{h.online ? "online" : "offline"}
            </span>
          </div>

          {#if h.online}
            <div class="hc-stats">
              <div><b>{h.running}</b><span>running</span></div>
              <div><b>{h.stopped}</b><span>stopped</span></div>
              <div><b>{h.images}</b><span>images</span></div>
            </div>
            <div class="hc-meta">
              {h.cpus} CPU · {humanBytes(h.mem_total)} RAM · Docker {h.docker_version ?? "?"}
              {#if h.os}<br />{h.os}{/if}
            </div>
          {:else}
            <div class="hc-meta" style="color:var(--red)">{h.error ?? "Non raggiungibile"}</div>
          {/if}
        </div>
      {/each}
    </div>

    {#if filtered.length === 0}
      <div class="empty">Nessun host corrisponde alla ricerca.</div>
    {/if}
  {/if}
</div>
