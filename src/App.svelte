<script lang="ts">
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api } from "./lib/api";
  import type { Host, Settings, View } from "./lib/types";
  import Dashboard from "./lib/Dashboard.svelte";
  import Containers from "./lib/Containers.svelte";
  import Images from "./lib/Images.svelte";
  import Volumes from "./lib/Volumes.svelte";
  import Networks from "./lib/Networks.svelte";
  import Stats from "./lib/Stats.svelte";
  import Metrics from "./lib/Metrics.svelte";
  import Events from "./lib/Events.svelte";
  import Maintenance from "./lib/Maintenance.svelte";
  import Schedules from "./lib/Schedules.svelte";
  import CommandPalette from "./lib/CommandPalette.svelte";

  let hosts = $state<Host[]>([]);
  let hostId = $state("local");
  let view = $state<View>("dashboard");
  let query = $state("");
  let settings = $state<Settings>({
    theme: "light",
    read_only: false,
    favorite_containers: [],
    alerts_enabled: false,
    alert_poll_secs: 30,
    metrics_enabled: true,
  });
  let showSettings = $state(false);
  let reloadKey = $state(0);

  let newName = $state("");
  let newEndpoint = $state("");
  let formError = $state<string | null>(null);
  let testing = $state(false);
  let testMsg = $state<{ ok: boolean; text: string } | null>(null);
  let hostTest = $state<Record<string, { ok: boolean; text: string; busy: boolean }>>({});

  const NAV: { id: View; label: string; ico: string }[] = [
    { id: "dashboard", label: "Dashboard", ico: "◉" },
    { id: "containers", label: "Container", ico: "▤" },
    { id: "stats", label: "Live Stats", ico: "◧" },
    { id: "metrics", label: "Metriche", ico: "◠" },
    { id: "images", label: "Immagini", ico: "◈" },
    { id: "volumes", label: "Volumi", ico: "⛁" },
    { id: "networks", label: "Reti", ico: "⇄" },
    { id: "events", label: "Eventi", ico: "⧗" },
    { id: "schedules", label: "Pianificazioni", ico: "⏱" },
    { id: "maintenance", label: "Manutenzione", ico: "⚑" },
  ];

  const titles: Record<View, string> = {
    dashboard: "Dashboard",
    containers: "Container",
    stats: "Live Stats",
    metrics: "Metriche",
    images: "Immagini",
    volumes: "Volumi",
    networks: "Reti",
    events: "Eventi",
    schedules: "Pianificazioni",
    maintenance: "Manutenzione",
  };

  const hostName = $derived(
    hosts.find((h) => h.id === hostId)?.name ?? hostId,
  );

  const subtitles = $derived<Record<View, string>>({
    dashboard: "Panoramica di tutti i tuoi host Docker",
    containers: `Container su ${hostName}`,
    stats: `Utilizzo risorse in tempo reale · ${hostName}`,
    metrics: `Storico CPU e memoria · ${hostName}`,
    images: `Immagini su ${hostName}`,
    volumes: `Volumi su ${hostName}`,
    networks: `Reti su ${hostName}`,
    events: `Timeline eventi Docker · ${hostName}`,
    schedules: "Azioni pianificate su tutti gli host",
    maintenance: `Disk usage e pulizia · ${hostName}`,
  });

  const avatar = $derived((hostName?.[0] ?? "D").toUpperCase());

  $effect(() => {
    document.documentElement.setAttribute("data-theme", settings.theme);
  });

  onMount(async () => {
    settings = await api.getSettings();
    hosts = await api.listHosts();
  });

  async function toggleTheme() {
    settings = { ...settings, theme: settings.theme === "dark" ? "light" : "dark" };
    await api.saveSettings(settings);
  }
  async function toggleReadOnly() {
    settings = { ...settings, read_only: !settings.read_only };
    await api.saveSettings(settings);
  }
  async function toggleAlerts() {
    settings = { ...settings, alerts_enabled: !settings.alerts_enabled };
    await api.saveSettings(settings);
  }
  async function saveInterval(v: number) {
    settings = { ...settings, alert_poll_secs: Math.max(5, v || 30) };
    await api.saveSettings(settings);
  }
  async function testConnection() {
    testMsg = null;
    formError = null;
    if (!newEndpoint.trim()) {
      formError = "Inserisci un endpoint da testare";
      return;
    }
    testing = true;
    try {
      const info = await api.testHost(newName, newEndpoint);
      testMsg = { ok: true, text: info };
    } catch (e) {
      testMsg = { ok: false, text: String(e) };
    } finally {
      testing = false;
    }
  }

  async function testHostRow(id: string, name: string, endpoint: string) {
    hostTest = { ...hostTest, [id]: { ok: false, text: "", busy: true } };
    try {
      const info = await api.testHost(name, endpoint);
      hostTest = { ...hostTest, [id]: { ok: true, text: info, busy: false } };
    } catch (e) {
      hostTest = { ...hostTest, [id]: { ok: false, text: String(e), busy: false } };
    }
  }

  async function addHost() {
    formError = null;
    try {
      hosts = await api.addHost(newName, newEndpoint);
      newName = "";
      newEndpoint = "";
      testMsg = null;
    } catch (e) {
      formError = String(e);
    }
  }
  async function removeHost(id: string) {
    hosts = await api.removeHost(id);
    if (hostId === id) hostId = "local";
  }

  let backupMsg = $state<{ ok: boolean; text: string } | null>(null);

  async function exportConfig() {
    backupMsg = null;
    try {
      const path = await save({
        defaultPath: "dockone-backup.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      await api.exportConfig(path);
      backupMsg = { ok: true, text: `Configurazione esportata in ${path}` };
    } catch (e) {
      backupMsg = { ok: false, text: String(e) };
    }
  }

  async function importConfig() {
    backupMsg = null;
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path || typeof path !== "string") return;
      await api.importConfig(path);
      hosts = await api.listHosts();
      settings = await api.getSettings();
      if (!hosts.some((h) => h.id === hostId)) hostId = "local";
      backupMsg = { ok: true, text: "Configurazione importata con successo" };
    } catch (e) {
      backupMsg = { ok: false, text: String(e) };
    }
  }
  function refresh() {
    reloadKey++;
  }

  let paletteOpen = $state(false);

  function onKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      paletteOpen = !paletteOpen;
    }
  }

  function paletteNavigate(hid: string | null, v: View, filter?: string) {
    if (hid) hostId = hid;
    view = v;
    query = filter ?? "";
    reloadKey++;
  }

  async function paletteAction(hid: string, id: string, action: string) {
    try {
      await api.containerAction(hid, id, action);
    } catch (e) {
      formError = String(e);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app">
  <div class="window">
    <header class="header">
      <div class="brand">
        <img src="/assets/icon.svg" alt="DockOne" />
        <b>Dock<span>One</span></b>
      </div>
      <div class="search-wrap">
        <span class="search-ico">⌕</span>
        <input class="search" placeholder="Cerca host, container, immagini…" bind:value={query} />
      </div>
      <div class="header-actions">
        {#if settings.read_only}
          <span class="badge offline"><span class="dot"></span>sola lettura</span>
        {/if}
        <button class="icon-btn" title="Comandi (Ctrl/⌘ K)" style="width:auto;padding:0 10px;font-size:12px;font-weight:600;letter-spacing:0.02em" onclick={() => (paletteOpen = true)}>⌘K</button>
        <button class="icon-btn" title="Aggiorna" onclick={refresh}>⟳</button>
        <button class="icon-btn" title="Tema" onclick={toggleTheme}>
          {settings.theme === "dark" ? "☀" : "☾"}
        </button>
        <button class="icon-btn" title="Impostazioni" onclick={() => (showSettings = true)}>⚙</button>
        <div class="user-chip">{avatar}</div>
      </div>
    </header>

    <div class="body">
      <aside class="sidebar">
        <button class="pill-btn" onclick={() => (showSettings = true)}>＋ Aggiungi host</button>

        <div class="host-select">
          <label for="host">Host attivo</label>
          <select id="host" bind:value={hostId}>
            {#each hosts as h (h.id)}
              <option value={h.id}>{h.favorite ? "★ " : ""}{h.name}</option>
            {/each}
          </select>
        </div>

        {#each NAV as n (n.id)}
          <button class="nav-item {view === n.id ? 'active' : ''}" onclick={() => (view = n.id)}>
            <span class="ico">{n.ico}</span>{n.label}
          </button>
        {/each}

        <div class="nav-spacer"></div>

        <div class="side-foot">
          <b>{hosts.length}</b> host configurati<br />
          DockOne · v0.7.0
        </div>
      </aside>

      <main class="content">
        <div class="page-head">
          <h1>{titles[view]}</h1>
          <p class="page-sub">{subtitles[view]}</p>
        </div>

        {#key `${view}-${hostId}-${reloadKey}`}
          {#if view === "dashboard"}
            <Dashboard {query} />
          {:else if view === "containers"}
            <Containers {hostId} {query} {settings} onSettings={(s) => (settings = s)} />
          {:else if view === "stats"}
            <Stats {hostId} {query} />
          {:else if view === "metrics"}
            <Metrics {hostId} {query} />
          {:else if view === "images"}
            <Images {hostId} {query} />
          {:else if view === "volumes"}
            <Volumes {hostId} {query} />
          {:else if view === "networks"}
            <Networks {hostId} {query} />
          {:else if view === "events"}
            <Events {hostId} {query} />
          {:else if view === "schedules"}
            <Schedules {hosts} />
          {:else if view === "maintenance"}
            <Maintenance {hostId} />
          {/if}
        {/key}
      </main>
    </div>
  </div>
</div>

{#if showSettings}
  <div class="overlay" role="presentation" onclick={() => (showSettings = false)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} style="max-width:580px">
      <div class="modal-head">
        <b>Impostazioni</b>
        <button class="icon-btn" onclick={() => (showSettings = false)}>✕</button>
      </div>
      <div class="modal-body" style="padding:22px">
        <div class="section-title">Host Docker</div>
        <table class="table" style="margin-bottom:20px">
          <tbody>
            {#each hosts as h (h.id)}
              <tr>
                <td class="row-title">{h.favorite ? "★ " : ""}{h.name}</td>
                <td class="mono">
                  {h.endpoint}
                  {#if hostTest[h.id] && !hostTest[h.id].busy}
                    <div style="margin-top:4px;color:{hostTest[h.id].ok ? 'var(--green)' : 'var(--red)'}">
                      {hostTest[h.id].ok ? "✓ " : "✗ "}{hostTest[h.id].text}
                    </div>
                  {/if}
                </td>
                <td style="text-align:right;white-space:nowrap">
                  <button class="act" disabled={hostTest[h.id]?.busy} onclick={() => testHostRow(h.id, h.name, h.endpoint)}>
                    {hostTest[h.id]?.busy ? "…" : "Testa"}
                  </button>
                  {#if h.id !== "local"}
                    <button class="act danger" onclick={() => removeHost(h.id)}>Rimuovi</button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>

        <div class="section-title">Aggiungi host remoto</div>
        {#if formError}<div class="err-banner">{formError}</div>{/if}
        <div class="host-form">
          <input placeholder="Nome (es. Production)" bind:value={newName} />
          <input placeholder="tcp://10.0.0.5:2375" bind:value={newEndpoint} />
          <button class="btn ghost" disabled={testing} onclick={testConnection}>{testing ? "Test…" : "Testa"}</button>
          <button class="btn" onclick={addHost}>Aggiungi</button>
        </div>
        {#if testMsg}
          <div class={testMsg.ok ? "ok-banner" : "err-banner"} style="margin-top:10px">
            {testMsg.ok ? "✓ Connesso · " : "✗ "}{testMsg.text}
          </div>
        {/if}
        <p class="meta">
          Endpoint supportati: <span class="mono">local</span>,
          <span class="mono">tcp://host:2375</span>,
          <span class="mono">unix:///path/docker.sock</span>,
          <span class="mono">ssh://user@host</span> (tunnel automatico, richiede chiave SSH).
          Il test tcp usa la porta 2375 se non la specifichi.
        </p>

        <div class="section-title" style="margin-top:22px">Preferenze</div>
        <label class="toggle" style="margin-bottom:12px">
          <input type="checkbox" style="width:auto" checked={settings.read_only} onchange={toggleReadOnly} />
          Modalità sola lettura (blocca start/stop/restart)
        </label>
        <label class="toggle">
          <input type="checkbox" style="width:auto" checked={settings.theme === "dark"} onchange={toggleTheme} />
          Tema scuro
        </label>

        <div class="section-title" style="margin-top:22px">Alerts</div>
        <label class="toggle" style="margin-bottom:12px">
          <input type="checkbox" style="width:auto" checked={settings.alerts_enabled} onchange={toggleAlerts} />
          Notifica crash / stato unhealthy dei container
        </label>
        <label class="toggle">
          Intervallo di controllo (secondi):
          <input
            type="number"
            min="5"
            style="width:90px"
            value={settings.alert_poll_secs}
            onchange={(e) => saveInterval(parseInt(e.currentTarget.value, 10))}
          />
        </label>

        <div class="section-title" style="margin-top:22px">Backup configurazione</div>
        {#if backupMsg}
          <div class={backupMsg.ok ? "ok-banner" : "err-banner"}>{backupMsg.ok ? "✓ " : "✗ "}{backupMsg.text}</div>
        {/if}
        <div style="display:flex;gap:8px">
          <button class="btn ghost" onclick={exportConfig}>⭳ Esporta…</button>
          <button class="btn ghost" onclick={importConfig}>⭱ Importa…</button>
        </div>
        <p class="meta">Salva o ripristina host e impostazioni in un file JSON.</p>
      </div>
    </div>
  </div>
{/if}

{#if paletteOpen}
  <CommandPalette
    views={NAV}
    {hosts}
    onClose={() => (paletteOpen = false)}
    onNavigate={paletteNavigate}
    onAction={paletteAction}
  />
{/if}
