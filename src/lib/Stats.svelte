<script lang="ts">
  import { onDestroy } from "svelte";
  import { api, humanBytes } from "./api";
  import type { Stat } from "./types";

  let { hostId, query = "" }: { hostId: string; query?: string } = $props();

  let items = $state<Stat[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let live = $state(true);
  let timer: ReturnType<typeof setInterval> | null = null;

  async function load() {
    try {
      items = await api.containerStats(hostId);
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function stop() {
    if (timer) clearInterval(timer);
    timer = null;
  }

  $effect(() => {
    hostId;
    loading = true;
    load();
    stop();
    if (live) timer = setInterval(load, 2500);
    return stop;
  });

  onDestroy(stop);

  const filtered = $derived(
    items
      .filter((s) => s.name.toLowerCase().includes(query.trim().toLowerCase()))
      .sort((a, b) => b.cpu_percent - a.cpu_percent),
  );

  function bar(pct: number, color: string) {
    const w = Math.min(100, Math.max(0, pct));
    return `background:linear-gradient(90deg, ${color} ${w}%, transparent ${w}%)`;
  }
</script>

<div>
  <div style="display:flex;justify-content:flex-end;margin-bottom:12px">
    <label class="toggle">
      <input type="checkbox" style="width:auto" bind:checked={live} />
      Aggiornamento live (2.5s)
    </label>
  </div>

  {#if error}<div class="err-banner">{error}</div>{/if}
  {#if loading}
    <div class="loading">Raccolta statistiche…</div>
  {:else if filtered.length === 0}
    <div class="empty">Nessun container in esecuzione.</div>
  {:else}
    <table class="table">
      <thead>
        <tr>
          <th>Container</th>
          <th style="width:180px">CPU %</th>
          <th style="width:220px">Memoria</th>
          <th>Rete (↓/↑)</th>
          <th>Disco (R/W)</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as s (s.id)}
          <tr>
            <td class="row-title">{s.name}</td>
            <td>
              <div class="metercell" style={bar(s.cpu_percent, "var(--accent-soft)")}>
                <span class="mono">{s.cpu_percent.toFixed(1)}%</span>
              </div>
            </td>
            <td>
              <div class="metercell" style={bar(s.mem_percent, "rgba(23,169,92,0.18)")}>
                <span class="mono">{humanBytes(s.mem_used)} / {humanBytes(s.mem_limit)}</span>
              </div>
            </td>
            <td class="mono">{humanBytes(s.net_rx)} / {humanBytes(s.net_tx)}</td>
            <td class="mono">{humanBytes(s.blk_read)} / {humanBytes(s.blk_write)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .metercell {
    border-radius: 7px;
    padding: 5px 9px;
    display: flex;
    align-items: center;
  }
</style>
