<script lang="ts">
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, humanBytes, type FsEntry } from "./api";
  import type { Container } from "./types";

  let {
    hostId,
    container,
    onClose,
  }: { hostId: string; container: Container; onClose: () => void } = $props();

  let path = $state("/");
  let entries = $state<FsEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let busy = $state<string | null>(null);

  let previewName = $state<string | null>(null);
  let previewText = $state("");

  async function load() {
    loading = true;
    error = null;
    previewName = null;
    try {
      entries = await api.fsList(hostId, container.id, path);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    path;
    load();
  });

  function join(dir: string, name: string): string {
    return dir.endsWith("/") ? dir + name : `${dir}/${name}`;
  }
  function up() {
    if (path === "/") return;
    const trimmed = path.replace(/\/+$/, "");
    const parent = trimmed.slice(0, trimmed.lastIndexOf("/")) || "/";
    path = parent;
  }
  function crumbs(): { label: string; path: string }[] {
    const out = [{ label: "/", path: "/" }];
    let acc = "";
    for (const seg of path.split("/").filter(Boolean)) {
      acc += "/" + seg;
      out.push({ label: seg, path: acc });
    }
    return out;
  }

  async function openEntry(e: FsEntry) {
    if (e.kind === "dir") {
      path = join(path, e.name);
    } else {
      previewName = e.name;
      previewText = "Caricamento…";
      try {
        const txt = await api.fsRead(hostId, container.id, join(path, e.name));
        previewText = txt.length > 200000 ? txt.slice(0, 200000) + "\n…(troncato)" : txt || "(vuoto)";
      } catch (err) {
        previewText = String(err);
      }
    }
  }

  async function download(e: FsEntry) {
    const dest = await save({ defaultPath: e.name });
    if (!dest) return;
    busy = e.name;
    try {
      await api.fsDownload(hostId, container.id, join(path, e.name), dest);
    } catch (err) {
      error = String(err);
    } finally {
      busy = null;
    }
  }

  async function upload() {
    const src = await open({ multiple: false, directory: false });
    if (!src || typeof src !== "string") return;
    busy = "__upload__";
    error = null;
    try {
      await api.fsUpload(hostId, container.id, src, path);
      await load();
    } catch (err) {
      error = String(err);
    } finally {
      busy = null;
    }
  }
</script>

<div class="overlay" role="presentation" onclick={onClose}>
  <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} style="width:820px;max-width:96vw">
    <div class="modal-head">
      <b>File · {container.name}</b>
      <div style="display:flex;gap:8px;align-items:center">
        <button class="act" disabled={busy === '__upload__'} onclick={upload}>{busy === '__upload__' ? "Carico…" : "⭱ Carica qui"}</button>
        <button class="icon-btn" onclick={onClose}>✕</button>
      </div>
    </div>
    <div class="modal-body" style="padding:0">
      <div class="crumbbar">
        <button class="act" onclick={up} disabled={path === '/'}>↑</button>
        {#each crumbs() as c (c.path)}
          <button class="crumb" onclick={() => (path = c.path)}>{c.label}</button><span class="sep">/</span>
        {/each}
      </div>

      {#if error}<div class="err-banner" style="margin:12px">{error}</div>{/if}

      {#if previewName}
        <div class="previewhead">
          <b>{previewName}</b>
          <button class="act" onclick={() => (previewName = null)}>← indietro</button>
        </div>
        <pre class="logs" style="max-height:56vh;overflow:auto">{previewText}</pre>
      {:else if loading}
        <div class="loading">Lettura directory…</div>
      {:else if entries.length === 0}
        <div class="empty">Directory vuota.</div>
      {:else}
        <div class="fslist">
          {#each entries as e (e.name)}
            <div class="fsrow">
              <button class="fsname" onclick={() => openEntry(e)}>
                <span class="fsico">{e.kind === "dir" ? "📁" : e.kind === "link" ? "🔗" : "📄"}</span>
                {e.name}
              </button>
              <span class="mono fssize">{e.kind === "file" ? humanBytes(e.size) : ""}</span>
              {#if e.kind !== "dir"}
                <button class="act" disabled={busy === e.name} onclick={() => download(e)}>Scarica</button>
              {:else}
                <span style="width:60px"></span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .crumbbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }
  .crumb {
    color: var(--accent);
    font-weight: 600;
    padding: 2px 4px;
    border-radius: 6px;
  }
  .crumb:hover {
    background: var(--row-hover);
  }
  .sep {
    color: var(--text-faint);
  }
  .fslist {
    max-height: 56vh;
    overflow-y: auto;
    padding: 6px;
  }
  .fsrow {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 10px;
    border-radius: 8px;
  }
  .fsrow:hover {
    background: var(--row-hover);
  }
  .fsname {
    flex: 1;
    text-align: left;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
  }
  .fsico {
    width: 18px;
  }
  .fssize {
    color: var(--text-faint);
    font-size: 12px;
    width: 80px;
    text-align: right;
  }
  .previewhead {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
  }
</style>
