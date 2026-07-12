<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import type { Container } from "./types";

  let {
    hostId,
    container,
    onClose,
  }: { hostId: string; container: Container; onClose: () => void } = $props();

  let el: HTMLDivElement;
  let term: Terminal | null = null;
  let fit: FitAddon | null = null;
  let session = "";
  let error = $state<string | null>(null);
  let unlisteners: UnlistenFn[] = [];
  let resizeObs: ResizeObserver | null = null;
  let shell = $state("/bin/sh");

  function sendResize() {
    if (!session || !term) return;
    invoke("term_resize", { session, rows: term.rows, cols: term.cols }).catch(() => {});
  }

  async function start() {
    error = null;
    term = new Terminal({
      fontSize: 13,
      fontFamily: '"JetBrains Mono", ui-monospace, Menlo, monospace',
      cursorBlink: true,
      theme: { background: "#0e1016", foreground: "#e9ecf6", cursor: "#2f8bff" },
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);
    fit.fit();

    try {
      session = await invoke<string>("term_start", { hostId, id: container.id, shell });
    } catch (e) {
      error = String(e);
      return;
    }

    unlisteners.push(
      await listen<number[]>(`term-data-${session}`, (ev) => {
        term?.write(new Uint8Array(ev.payload));
      }),
    );
    unlisteners.push(
      await listen(`term-exit-${session}`, () => {
        term?.write("\r\n\x1b[2m— sessione terminata —\x1b[0m\r\n");
      }),
    );

    term.onData((d) => {
      invoke("term_write", { session, data: d }).catch(() => {});
    });

    resizeObs = new ResizeObserver(() => {
      fit?.fit();
      sendResize();
    });
    resizeObs.observe(el);
    sendResize();
    term.focus();
  }

  async function cleanup() {
    resizeObs?.disconnect();
    for (const u of unlisteners) u();
    unlisteners = [];
    if (session) {
      await invoke("term_close", { session }).catch(() => {});
      session = "";
    }
    term?.dispose();
    term = null;
  }

  async function restart() {
    await cleanup();
    await start();
  }

  onMount(start);
  onDestroy(cleanup);

  async function close() {
    await cleanup();
    onClose();
  }
</script>

<div class="overlay" role="presentation" onclick={close}>
  <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} style="width:920px;max-width:96vw">
    <div class="modal-head">
      <b>Terminal · {container.name}</b>
      <div style="display:flex;align-items:center;gap:8px">
        <select bind:value={shell} onchange={restart} style="width:auto;padding:5px 8px;font-size:12px">
          <option value="/bin/sh">/bin/sh</option>
          <option value="/bin/bash">/bin/bash</option>
          <option value="/bin/ash">/bin/ash</option>
          <option value="/bin/zsh">/bin/zsh</option>
        </select>
        <button class="icon-btn" title="Chiudi" onclick={close}>✕</button>
      </div>
    </div>
    <div class="modal-body" style="padding:0">
      {#if error}<div class="err-banner" style="margin:14px">{error}</div>{/if}
      <div bind:this={el} class="termbox"></div>
    </div>
  </div>
</div>

<style>
  .termbox {
    height: 62vh;
    padding: 8px 4px 8px 10px;
    background: #0e1016;
  }
</style>
