<script lang="ts">
  import { api } from "./api";
  import type { DockerEvent } from "./types";

  let { hostId, query = "" }: { hostId: string; query?: string } = $props();

  let items = $state<DockerEvent[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      items = await api.recentEvents(hostId);
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

  function when(unix: number): string {
    if (!unix) return "—";
    const d = new Date(unix * 1000);
    return d.toLocaleString();
  }

  const filtered = $derived(
    items.filter((e) => {
      const q = query.trim().toLowerCase();
      if (!q) return true;
      return (
        e.actor.toLowerCase().includes(q) ||
        e.action.toLowerCase().includes(q) ||
        e.kind.toLowerCase().includes(q)
      );
    }),
  );

  function tone(action: string): string {
    const a = action.toLowerCase();
    if (a.includes("die") || a.includes("kill") || a.includes("destroy") || a.includes("oom"))
      return "exited";
    if (a.includes("start") || a.includes("create") || a.includes("pull") || a.includes("health_status: healthy"))
      return "running";
    if (a.includes("stop") || a.includes("pause") || a.includes("restart")) return "paused";
    return "offline";
  }
</script>

<div>
  {#if error}<div class="err-banner">{error}</div>{/if}
  {#if loading}
    <div class="loading">Caricamento eventi (ultime 6 ore)…</div>
  {:else if filtered.length === 0}
    <div class="empty">Nessun evento nelle ultime 6 ore.</div>
  {:else}
    <div class="timeline">
      {#each filtered as e, i (i)}
        <div class="tl-row">
          <span class="badge {tone(e.action)}"><span class="dot"></span>{e.kind}</span>
          <b class="tl-action">{e.action}</b>
          <span class="tl-actor mono">{e.actor}</span>
          <span class="tl-time mono">{when(e.time)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .timeline {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 16px;
    overflow: hidden;
    box-shadow: var(--shadow-card);
  }
  .tl-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 11px 16px;
    border-bottom: 1px solid var(--border);
  }
  .tl-row:last-child {
    border-bottom: none;
  }
  .tl-action {
    min-width: 150px;
    font-size: 13px;
  }
  .tl-actor {
    flex: 1;
    color: var(--text-dim);
  }
  .tl-time {
    color: var(--text-faint);
    font-size: 11.5px;
  }
</style>
