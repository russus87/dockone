<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { save } from "@tauri-apps/plugin-dialog";
  import { api } from "./api";
  import type { Container } from "./types";

  let {
    hostId,
    container,
    onClose,
  }: { hostId: string; container: Container; onClose: () => void } = $props();

  let lines = $state<string[]>([]);
  let session = "";
  let unlisteners: UnlistenFn[] = [];
  let search = $state("");
  let follow = $state(true);
  let error = $state<string | null>(null);
  let box: HTMLPreElement;

  const filtered = $derived(
    search.trim()
      ? lines.filter((l) => l.toLowerCase().includes(search.trim().toLowerCase()))
      : lines,
  );

  async function scrollBottom() {
    await tick();
    if (box) box.scrollTop = box.scrollHeight;
  }

  onMount(async () => {
    try {
      session = await api.logStart(hostId, container.id);
    } catch (e) {
      error = String(e);
      return;
    }
    unlisteners.push(
      await listen<string>(`log-data-${session}`, (ev) => {
        const parts = String(ev.payload).split("\n").filter((p) => p.length > 0);
        if (parts.length) {
          lines = [...lines, ...parts].slice(-5000);
          if (follow) scrollBottom();
        }
      }),
    );
    unlisteners.push(
      await listen(`log-exit-${session}`, () => {
        lines = [...lines, "— stream terminato —"];
      }),
    );
  });

  async function cleanup() {
    for (const u of unlisteners) u();
    unlisteners = [];
    if (session) {
      await api.logStop(session).catch(() => {});
      session = "";
    }
  }
  onDestroy(cleanup);

  async function close() {
    await cleanup();
    onClose();
  }

  async function download() {
    const path = await save({
      defaultPath: `${container.name}.log`,
      filters: [{ name: "Log", extensions: ["log", "txt"] }],
    });
    if (path) await api.saveText(path, lines.join("\n")).catch((e) => (error = String(e)));
  }
</script>

<div class="overlay" role="presentation" onclick={close}>
  <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} style="width:900px;max-width:96vw">
    <div class="modal-head">
      <b>Logs · {container.name}</b>
      <div style="display:flex;align-items:center;gap:8px">
        <input placeholder="Filtra…" bind:value={search} style="width:200px;padding:6px 10px;font-size:12px" />
        <label class="toggle" style="font-size:12px"><input type="checkbox" style="width:auto" bind:checked={follow} /> segui</label>
        <button class="act" onclick={download}>Scarica</button>
        <button class="icon-btn" onclick={close}>✕</button>
      </div>
    </div>
    <div class="modal-body" style="padding:0">
      {#if error}<div class="err-banner" style="margin:14px">{error}</div>{/if}
      <pre class="logs" bind:this={box} style="height:60vh;overflow:auto">{filtered.join("\n") || "In attesa di output…"}</pre>
    </div>
  </div>
</div>
