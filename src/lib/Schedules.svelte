<script lang="ts">
  import { api } from "./api";
  import type { Host, Schedule } from "./types";

  let { hosts = [] }: { hosts?: Host[] } = $props();

  let items = $state<Schedule[]>([]);
  let error = $state<string | null>(null);

  // form
  let hostId = $state("local");
  let container = $state("");
  let action = $state("restart");
  let kind = $state("daily");
  let time = $state("03:00");
  let everyMin = $state(60);

  async function load() {
    try {
      items = await api.listSchedules();
    } catch (e) {
      error = String(e);
    }
  }
  $effect(() => {
    load();
  });

  async function add() {
    error = null;
    if (!container.trim()) {
      error = "Indica il nome del container";
      return;
    }
    const host = hosts.find((h) => h.id === hostId);
    const s: Schedule = {
      id: "",
      host_id: hostId,
      host_name: host?.name ?? hostId,
      container: container.trim(),
      action,
      kind,
      time,
      every_min: Number(everyMin) || 0,
      enabled: true,
      last_run: 0,
    };
    try {
      items = await api.addSchedule(s);
      container = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function toggle(id: string) {
    items = await api.toggleSchedule(id);
  }
  async function remove(id: string) {
    items = await api.removeSchedule(id);
  }

  function when(s: Schedule): string {
    return s.kind === "daily" ? `ogni giorno alle ${s.time}` : `ogni ${s.every_min} min`;
  }
  function lastRun(s: Schedule): string {
    if (!s.last_run) return "mai";
    return new Date(s.last_run * 1000).toLocaleString();
  }
</script>

<div>
  {#if error}<div class="err-banner">{error}</div>{/if}

  <div class="card" style="margin-bottom:20px">
    <div class="section-title">Nuova pianificazione</div>
    <div class="schedrow">
      <select bind:value={hostId}>
        {#each hosts as h (h.id)}<option value={h.id}>{h.name}</option>{/each}
      </select>
      <input placeholder="container (nome)" bind:value={container} />
      <select bind:value={action}>
        <option value="start">start</option>
        <option value="stop">stop</option>
        <option value="restart">restart</option>
      </select>
      <select bind:value={kind}>
        <option value="daily">giornaliero</option>
        <option value="interval">a intervallo</option>
      </select>
      {#if kind === "daily"}
        <input type="time" bind:value={time} />
      {:else}
        <input type="number" min="1" style="width:110px" bind:value={everyMin} title="minuti" />
      {/if}
      <button class="btn" onclick={add}>Aggiungi</button>
    </div>
  </div>

  {#if items.length === 0}
    <div class="empty">Nessuna pianificazione. Aggiungine una qui sopra.</div>
  {:else}
    <table class="table">
      <thead>
        <tr>
          <th>Attiva</th>
          <th>Host</th>
          <th>Container</th>
          <th>Azione</th>
          <th>Quando</th>
          <th>Ultima esecuzione</th>
          <th style="text-align:right"></th>
        </tr>
      </thead>
      <tbody>
        {#each items as s (s.id)}
          <tr>
            <td><input type="checkbox" style="width:auto" checked={s.enabled} onchange={() => toggle(s.id)} /></td>
            <td class="row-title">{s.host_name}</td>
            <td class="mono">{s.container}</td>
            <td><span class="badge {s.action === 'stop' ? 'exited' : s.action === 'start' ? 'running' : 'paused'}">{s.action}</span></td>
            <td>{when(s)}</td>
            <td class="mono">{lastRun(s)}</td>
            <td style="text-align:right">
              <button class="act danger" onclick={() => remove(s.id)}>Rimuovi</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .schedrow {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }
  .schedrow select,
  .schedrow input {
    width: auto;
    flex: 0 0 auto;
  }
  .schedrow input[placeholder] {
    flex: 1;
    min-width: 160px;
  }
</style>
