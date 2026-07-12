<script lang="ts">
  import { onDestroy } from "svelte";
  import { api, humanBytes } from "./api";
  import type { Sample, Series } from "./types";

  let { hostId, query = "" }: { hostId: string; query?: string } = $props();

  let series = $state<Series[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;

  async function load() {
    try {
      series = await api.metricsHistory(hostId);
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    hostId;
    loading = true;
    load();
    if (timer) clearInterval(timer);
    timer = setInterval(load, 5000);
    return () => {
      if (timer) clearInterval(timer);
    };
  });
  onDestroy(() => {
    if (timer) clearInterval(timer);
  });

  const W = 300;
  const H = 46;

  function line(points: Sample[], pick: (p: Sample) => number, max: number): string {
    if (points.length < 2) return "";
    const n = points.length;
    return points
      .map((p, i) => {
        const x = (i / (n - 1)) * W;
        const v = Math.max(0, Math.min(max, pick(p)));
        const y = H - (v / max) * H;
        return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }

  const cpu = (p: Sample) => p.cpu;
  const memPct = (p: Sample) => (p.mem_limit > 0 ? (p.mem / p.mem_limit) * 100 : 0);

  const filtered = $derived(
    series.filter((s) => s.name.toLowerCase().includes(query.trim().toLowerCase())),
  );

  function cpuMax(s: Series): number {
    return Math.max(100, ...s.points.map((p) => p.cpu));
  }
  function last(s: Series): Sample | null {
    return s.points.length ? s.points[s.points.length - 1] : null;
  }
</script>

<div>
  {#if error}<div class="err-banner">{error}</div>{/if}
  {#if loading}
    <div class="loading">Caricamento storico…</div>
  {:else if filtered.length === 0}
    <div class="empty">Nessuno storico ancora. Il campionamento parte quando visiti l'host (~ogni 10s).</div>
  {:else}
    <div class="grid">
      {#each filtered as s (s.name)}
        {@const lp = last(s)}
        <div class="card">
          <div class="card-head" style="margin-bottom:10px">
            <b>{s.name}</b>
            <span class="mono" style="font-size:11px;color:var(--text-faint)">{s.points.length} punti</span>
          </div>

          <div class="metric">
            <div class="mlabel"><span>CPU</span><b style="color:var(--accent)">{lp ? lp.cpu.toFixed(1) : "0"}%</b></div>
            <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" class="spark">
              <path d={line(s.points, cpu, cpuMax(s))} fill="none" stroke="var(--accent)" stroke-width="1.6" />
            </svg>
          </div>

          <div class="metric" style="margin-top:8px">
            <div class="mlabel">
              <span>Memoria</span>
              <b style="color:var(--green)">{lp ? humanBytes(lp.mem) : "0"}{lp && lp.mem_limit > 0 ? " / " + humanBytes(lp.mem_limit) : ""}</b>
            </div>
            <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" class="spark">
              <path d={line(s.points, memPct, 100)} fill="none" stroke="var(--green)" stroke-width="1.6" />
            </svg>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .card-head b {
    font-weight: 650;
  }
  .metric {
    background: var(--card-2);
    border-radius: 10px;
    padding: 8px 10px;
  }
  .mlabel {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-faint);
    margin-bottom: 4px;
  }
  .mlabel b {
    font-size: 14px;
    text-transform: none;
    letter-spacing: 0;
  }
  .spark {
    display: block;
    width: 100%;
    height: 46px;
  }
</style>
