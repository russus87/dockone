<script lang="ts">
  import { api } from "./api";
  import type { DeploySpec, PortMap, VolMap } from "./types";

  let {
    hostId,
    initial = null,
    title = "Deploy container",
    onClose,
    onDone,
  }: {
    hostId: string;
    initial?: DeploySpec | null;
    title?: string;
    onClose: () => void;
    onDone: () => void;
  } = $props();

  let name = $state(initial?.name ?? "");
  let image = $state(initial?.image ?? "");
  let cmd = $state((initial?.cmd ?? []).join(" "));
  let env = $state((initial?.env ?? []).join("\n"));
  let ports = $state<PortMap[]>(initial?.ports?.length ? [...initial.ports] : [{ host: "", container: "", proto: "tcp" }]);
  let volumes = $state<VolMap[]>(initial?.volumes?.length ? [...initial.volumes] : [{ host: "", container: "" }]);
  let restart = $state(initial?.restart ?? "no");
  let autostart = $state(true);

  let busy = $state(false);
  let error = $state<string | null>(null);

  function addPort() {
    ports = [...ports, { host: "", container: "", proto: "tcp" }];
  }
  function addVol() {
    volumes = [...volumes, { host: "", container: "" }];
  }

  async function submit() {
    error = null;
    if (!image.trim()) {
      error = "L'immagine è obbligatoria (es. nginx:latest)";
      return;
    }
    const spec: DeploySpec = {
      name: name.trim(),
      image: image.trim(),
      cmd: cmd.trim() ? cmd.trim().split(/\s+/) : [],
      env: env
        .split("\n")
        .map((l) => l.trim())
        .filter(Boolean),
      ports: ports.filter((p) => p.container.trim()),
      volumes: volumes.filter((v) => v.host.trim() && v.container.trim()),
      restart,
    };
    busy = true;
    try {
      await api.deployContainer(hostId, spec, autostart);
      onDone();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="overlay" role="presentation" onclick={onClose}>
  <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} style="max-width:640px">
    <div class="modal-head">
      <b>{title}</b>
      <button class="icon-btn" onclick={onClose}>✕</button>
    </div>
    <div class="modal-body" style="padding:20px">
      {#if error}<div class="err-banner">{error}</div>{/if}

      <div class="field">
        <label for="d-image">Immagine *</label>
        <input id="d-image" placeholder="nginx:latest" bind:value={image} />
      </div>
      <div class="field">
        <label for="d-name">Nome (opzionale)</label>
        <input id="d-name" placeholder="il-mio-container" bind:value={name} />
      </div>
      <div class="field">
        <label for="d-cmd">Comando (opzionale)</label>
        <input id="d-cmd" placeholder="es. nginx -g 'daemon off;'" bind:value={cmd} />
      </div>

      <div class="field">
        <span class="fl">Porte (host → container)</span>
        {#each ports as p, i (i)}
          <div class="rowinputs">
            <input placeholder="host (8080)" bind:value={p.host} />
            <span class="arrow">→</span>
            <input placeholder="container (80)" bind:value={p.container} />
            <select bind:value={p.proto} style="max-width:90px">
              <option value="tcp">tcp</option>
              <option value="udp">udp</option>
            </select>
          </div>
        {/each}
        <button class="miniadd" onclick={addPort}>＋ porta</button>
      </div>

      <div class="field">
        <span class="fl">Volumi (host → container)</span>
        {#each volumes as v, i (i)}
          <div class="rowinputs">
            <input placeholder="/host/path o nome-volume" bind:value={v.host} />
            <span class="arrow">→</span>
            <input placeholder="/container/path" bind:value={v.container} />
          </div>
        {/each}
        <button class="miniadd" onclick={addVol}>＋ volume</button>
      </div>

      <div class="field">
        <label for="d-env">Variabili d'ambiente (una per riga, KEY=value)</label>
        <textarea id="d-env" rows="3" placeholder={"TZ=Europe/Rome\nPUID=1000"} bind:value={env}></textarea>
      </div>

      <div class="field row2">
        <div style="flex:1">
          <label for="d-restart">Restart policy</label>
          <select id="d-restart" bind:value={restart}>
            <option value="no">no</option>
            <option value="on-failure">on-failure</option>
            <option value="always">always</option>
            <option value="unless-stopped">unless-stopped</option>
          </select>
        </div>
        <label class="toggle" style="align-self:flex-end;padding-bottom:9px">
          <input type="checkbox" style="width:auto" bind:checked={autostart} />
          Avvia subito
        </label>
      </div>

      <div style="display:flex;justify-content:flex-end;gap:8px;margin-top:8px">
        <button class="btn ghost" onclick={onClose}>Annulla</button>
        <button class="btn" disabled={busy} onclick={submit}>{busy ? "Creazione…" : "Deploy"}</button>
      </div>
    </div>
  </div>
</div>

<style>
  .field {
    margin-bottom: 14px;
  }
  .field label,
  .field .fl {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-dim);
    margin-bottom: 6px;
  }
  .field input,
  .field textarea,
  .field select {
    width: 100%;
    background: var(--card-2);
    border: 1px solid var(--border);
    border-radius: 9px;
    padding: 9px 12px;
    font-size: 13px;
    color: var(--text);
    outline: none;
    font-family: inherit;
  }
  .field input:focus,
  .field textarea:focus,
  .field select:focus {
    border-color: var(--accent);
  }
  .rowinputs {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .arrow {
    color: var(--text-faint);
  }
  .miniadd {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    padding: 4px 2px;
  }
  .row2 {
    display: flex;
    gap: 16px;
    align-items: flex-start;
  }
  .btn.ghost {
    background: var(--card-2);
    color: var(--text);
    border: 1px solid var(--border);
  }
</style>
