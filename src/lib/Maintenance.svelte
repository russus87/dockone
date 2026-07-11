<script lang="ts">
  import { api, humanBytes } from "./api";
  import type { Df } from "./types";

  let { hostId }: { hostId: string } = $props();

  let df = $state<Df | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let busy = $state<string | null>(null);
  let result = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      df = await api.systemDf(hostId);
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

  async function prune(kind: string, label: string) {
    if (!confirm(`Eseguire il prune di ${label} non utilizzati?`)) return;
    busy = kind;
    result = null;
    error = null;
    try {
      const r = await api.prune(hostId, kind);
      result = `${label}: ${r.detail}${r.reclaimed ? ` · ${humanBytes(r.reclaimed)} liberati` : ""}`;
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = null;
    }
  }

  const KINDS = [
    { kind: "containers", label: "Container fermi", ico: "▤" },
    { kind: "images", label: "Immagini dangling", ico: "◈" },
    { kind: "volumes", label: "Volumi orfani", ico: "⛁" },
    { kind: "networks", label: "Reti inutilizzate", ico: "⇄" },
  ];
</script>

<div>
  {#if error}<div class="err-banner">{error}</div>{/if}
  {#if result}<div class="ok-banner">✓ {result}</div>{/if}

  <p class="section-label">Utilizzo del disco</p>
  {#if loading}
    <div class="loading">Analisi spazio…</div>
  {:else if df}
    <div class="kpi-row">
      <div class="kpi"><b>{humanBytes(df.images_size)}</b><small>{df.images_count} immagini</small></div>
      <div class="kpi"><b>{humanBytes(df.volumes_size)}</b><small>{df.volumes_count} volumi</small></div>
      <div class="kpi"><b>{df.containers_count}</b><small>Container</small></div>
    </div>
  {/if}

  <p class="section-label" style="margin-top:26px">Pulizia (Prune)</p>
  <div class="grid">
    {#each KINDS as k (k.kind)}
      <div class="host-card">
        <div class="hc-top">
          <div class="hc-icon">{k.ico}</div>
          <div class="hc-name">{k.label}</div>
        </div>
        <button class="btn" style="width:100%;background:var(--red)" disabled={busy === k.kind} onclick={() => prune(k.kind, k.label)}>
          {busy === k.kind ? "In corso…" : "Prune"}
        </button>
      </div>
    {/each}
  </div>
</div>

<style>
  .ok-banner {
    background: rgba(23, 169, 92, 0.1);
    color: var(--green);
    border: 1px solid rgba(23, 169, 92, 0.28);
    border-radius: 11px;
    padding: 11px 15px;
    margin-bottom: 16px;
    font-size: 13px;
  }
</style>
