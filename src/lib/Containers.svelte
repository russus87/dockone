<script lang="ts">
  import { api, ago } from "./api";
  import type { Container, DeploySpec, Settings } from "./types";
  import DeployModal from "./DeployModal.svelte";

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
  let selected = $state<Record<string, boolean>>({});
  let bulkBusy = $state(false);

  let logsFor = $state<Container | null>(null);
  let logText = $state("");
  let inspectFor = $state<Container | null>(null);
  let inspectText = $state("");
  let execFor = $state<Container | null>(null);
  let execCmd = $state("");
  let execOut = $state("");
  let execBusy = $state(false);

  let deployOpen = $state(false);
  let deployInitial = $state<DeploySpec | null>(null);
  let deployTitle = $state("Deploy container");

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
    selected = {};
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

  async function remove(c: Container) {
    if (!confirm(`Rimuovere il container ${c.name}?`)) return;
    busy = { ...busy, [c.id]: true };
    try {
      await api.removeContainer(hostId, c.id);
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = { ...busy, [c.id]: false };
    }
  }

  function openDeploy() {
    deployInitial = null;
    deployTitle = "Deploy container";
    deployOpen = true;
  }

  async function clone(c: Container) {
    error = null;
    try {
      deployInitial = await api.containerConfig(hostId, c.id);
      deployTitle = `Clona · ${c.name}`;
      deployOpen = true;
    } catch (e) {
      error = String(e);
    }
  }

  function onDeployed() {
    deployOpen = false;
    load();
  }

  const selectedIds = $derived(Object.keys(selected).filter((id) => selected[id]));

  function toggleAll(on: boolean) {
    const next: Record<string, boolean> = {};
    if (on) for (const c of filtered) next[c.id] = true;
    selected = next;
  }

  async function bulk(action: string) {
    const ids = selectedIds;
    if (ids.length === 0) return;
    if (action === "remove" && !confirm(`Rimuovere ${ids.length} container?`)) return;
    bulkBusy = true;
    error = null;
    try {
      for (const id of ids) {
        if (action === "remove") await api.removeContainer(hostId, id);
        else await api.containerAction(hostId, id, action);
      }
      selected = {};
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      bulkBusy = false;
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

  function openExec(c: Container) {
    execFor = c;
    execCmd = "";
    execOut = "";
  }
  async function runExec() {
    if (!execFor || !execCmd.trim()) return;
    execBusy = true;
    try {
      execOut = (await api.execRun(hostId, execFor.id, execCmd)) || "(nessun output)";
    } catch (e) {
      execOut = String(e);
    } finally {
      execBusy = false;
    }
  }

  const filtered = $derived(
    items
      .filter((c) => {
        const q = query.trim().toLowerCase();
        if (!q) return true;
        return (
          c.name.toLowerCase().includes(q) ||
          c.image.toLowerCase().includes(q) ||
          (c.compose ?? "").toLowerCase().includes(q)
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

  const allChecked = $derived(filtered.length > 0 && filtered.every((c) => selected[c.id]));
</script>

<div>
  <div style="display:flex;justify-content:flex-end;margin-bottom:14px">
    <button class="btn" onclick={openDeploy}>＋ Deploy container</button>
  </div>

  {#if error}<div class="err-banner">{error}</div>{/if}

  {#if selectedIds.length > 0}
    <div class="bulkbar">
      <b>{selectedIds.length} selezionati</b>
      <button class="act primary" disabled={bulkBusy} onclick={() => bulk("start")}>Start</button>
      <button class="act" disabled={bulkBusy} onclick={() => bulk("restart")}>Restart</button>
      <button class="act danger" disabled={bulkBusy} onclick={() => bulk("stop")}>Stop</button>
      <button class="act danger" disabled={bulkBusy} onclick={() => bulk("remove")}>Rimuovi</button>
      <button class="act" onclick={() => (selected = {})}>Deseleziona</button>
    </div>
  {/if}

  {#if loading}
    <div class="loading">Caricamento container…</div>
  {:else if filtered.length === 0}
    <div class="empty">Nessun container.</div>
  {:else}
    <table class="table">
      <thead>
        <tr>
          <th style="width:28px"><input type="checkbox" style="width:auto" checked={allChecked} onchange={(e) => toggleAll(e.currentTarget.checked)} /></th>
          <th style="width:24px"></th>
          <th>Nome</th>
          <th>Immagine</th>
          <th>Stato</th>
          <th>Stack</th>
          <th>Porte</th>
          <th style="text-align:right">Azioni</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as c (c.id)}
          <tr>
            <td><input type="checkbox" style="width:auto" checked={!!selected[c.id]} onchange={(e) => (selected = { ...selected, [c.id]: e.currentTarget.checked })} /></td>
            <td>
              <button class="star {isFav(c) ? 'on' : ''}" title="Preferito" onclick={() => toggleFav(c)}>{isFav(c) ? "★" : "☆"}</button>
            </td>
            <td class="row-title">{c.name}</td>
            <td class="mono">{c.image}</td>
            <td>
              <span class="badge {c.state}"><span class="dot"></span>{c.state || "?"}</span>
              {#if c.health !== "none"}
                <span class="badge {c.health === 'healthy' ? 'running' : c.health === 'unhealthy' ? 'exited' : 'paused'}" style="margin-left:4px">{c.health}</span>
              {/if}
            </td>
            <td class="mono">{c.compose ?? "—"}</td>
            <td class="mono">{c.ports.join(", ") || "—"}</td>
            <td>
              <div class="actions" style="justify-content:flex-end;flex-wrap:wrap">
                {#if c.state === "running"}
                  <button class="act danger" disabled={busy[c.id]} onclick={() => act(c, "stop")}>Stop</button>
                  <button class="act" disabled={busy[c.id]} onclick={() => act(c, "restart")}>Restart</button>
                  <button class="act" disabled={busy[c.id]} onclick={() => act(c, "pause")}>Pause</button>
                  <button class="act danger" disabled={busy[c.id]} onclick={() => act(c, "kill")}>Kill</button>
                  <button class="act" onclick={() => openExec(c)}>Exec</button>
                {:else if c.state === "paused"}
                  <button class="act primary" disabled={busy[c.id]} onclick={() => act(c, "unpause")}>Unpause</button>
                {:else}
                  <button class="act primary" disabled={busy[c.id]} onclick={() => act(c, "start")}>Start</button>
                  <button class="act danger" disabled={busy[c.id]} onclick={() => remove(c)}>Rimuovi</button>
                {/if}
                <button class="act" onclick={() => openLogs(c)}>Logs</button>
                <button class="act" onclick={() => openInspect(c)}>Inspect</button>
                <button class="act" onclick={() => clone(c)}>Clona</button>
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if logsFor}
  <div class="overlay" role="presentation" onclick={() => (logsFor = null)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <div class="modal-head">
        <b>Logs · {logsFor.name}</b>
        <button class="icon-btn" onclick={() => (logsFor = null)}>✕</button>
      </div>
      <div class="modal-body"><pre class="logs">{logText}</pre></div>
    </div>
  </div>
{/if}

{#if inspectFor}
  <div class="overlay" role="presentation" onclick={() => (inspectFor = null)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <div class="modal-head">
        <b>Inspect · {inspectFor.name}</b>
        <button class="icon-btn" onclick={() => (inspectFor = null)}>✕</button>
      </div>
      <div class="modal-body"><pre class="json">{inspectText}</pre></div>
    </div>
  </div>
{/if}

{#if execFor}
  <div class="overlay" role="presentation" onclick={() => (execFor = null)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} style="max-width:720px">
      <div class="modal-head">
        <b>Exec · {execFor.name}</b>
        <button class="icon-btn" onclick={() => (execFor = null)}>✕</button>
      </div>
      <div class="modal-body" style="padding:18px">
        <div class="host-form" style="margin-top:0">
          <input placeholder="Comando — es. ls -la /  ·  cat /etc/os-release" bind:value={execCmd} onkeydown={(e) => e.key === "Enter" && runExec()} />
          <button class="btn" disabled={execBusy} onclick={runExec}>{execBusy ? "…" : "Esegui"}</button>
        </div>
        <p class="meta">Eseguito tramite <span class="mono">/bin/sh -c</span> nel container.</p>
        {#if execOut}<pre class="logs" style="margin-top:12px;border-radius:10px">{execOut}</pre>{/if}
      </div>
    </div>
  </div>
{/if}

{#if deployOpen}
  <DeployModal
    {hostId}
    initial={deployInitial}
    title={deployTitle}
    onClose={() => (deployOpen = false)}
    onDone={onDeployed}
  />
{/if}

<style>
  .bulkbar {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--accent-soft);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 10px 14px;
    margin-bottom: 14px;
  }
  .bulkbar b {
    margin-right: 6px;
    color: var(--accent);
  }
</style>
