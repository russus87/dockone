<script lang="ts">
  import { onMount } from "svelte";
  import { api, type FlatContainer } from "./api";
  import type { Host, View } from "./types";

  let {
    views,
    hosts,
    onClose,
    onNavigate,
    onAction,
  }: {
    views: { id: View; label: string }[];
    hosts: Host[];
    onClose: () => void;
    onNavigate: (hostId: string | null, view: View, filter?: string) => void;
    onAction: (hostId: string, id: string, action: string) => Promise<void>;
  } = $props();

  let query = $state("");
  let containers = $state<FlatContainer[]>([]);
  let loading = $state(true);
  let sel = $state(0);
  let inputEl: HTMLInputElement;

  onMount(async () => {
    inputEl?.focus();
    try {
      containers = await api.allContainers();
    } catch {
      /* ignore — palette still works for hosts/views */
    } finally {
      loading = false;
    }
  });

  type Item = {
    kind: "container" | "host" | "view";
    label: string;
    sub: string;
    view?: View;
    hostId?: string;
    id?: string;
    state?: string;
  };

  const results = $derived.by<Item[]>(() => {
    const q = query.trim().toLowerCase();
    const items: Item[] = [];
    for (const c of containers)
      items.push({
        kind: "container",
        label: c.name,
        sub: `${c.host_name} · ${c.image}`,
        hostId: c.host_id,
        id: c.id,
        state: c.state,
      });
    for (const h of hosts) items.push({ kind: "host", label: h.name, sub: "host", hostId: h.id });
    for (const v of views) items.push({ kind: "view", label: v.label, sub: "vista", view: v.id });
    const f = q
      ? items.filter((i) => `${i.label} ${i.sub}`.toLowerCase().includes(q))
      : items;
    return f.slice(0, 60);
  });

  $effect(() => {
    results;
    sel = 0;
  });

  function choose(i: Item) {
    if (i.kind === "container") onNavigate(i.hostId!, "containers", i.label);
    else if (i.kind === "host") onNavigate(i.hostId!, "containers");
    else if (i.kind === "view") onNavigate(null, i.view!);
    onClose();
  }

  function key(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = Math.min(sel + 1, results.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = Math.max(sel - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const it = results[sel];
      if (it) choose(it);
    } else if (e.key === "Escape") {
      onClose();
    }
  }

  async function act(it: Item, action: string, e: Event) {
    e.stopPropagation();
    if (it.hostId && it.id) {
      await onAction(it.hostId, it.id, action);
      containers = await api.allContainers().catch(() => containers);
    }
  }

  const glyph = { container: "▤", host: "🖳", view: "◉" };
</script>

<div class="overlay pal-overlay" role="presentation" onclick={onClose}>
  <div class="palette" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
    <input
      bind:this={inputEl}
      bind:value={query}
      onkeydown={key}
      class="pal-input"
      placeholder="Cerca container, host o vista…   (Esc per chiudere)"
    />
    <div class="pal-list">
      {#if loading}
        <div class="pal-empty">Caricamento container da tutti gli host…</div>
      {:else if results.length === 0}
        <div class="pal-empty">Nessun risultato.</div>
      {:else}
        {#each results as it, i (it.kind + (it.id ?? it.view ?? it.hostId) + i)}
          <button
            class="pal-item {i === sel ? 'on' : ''}"
            onmousemove={() => (sel = i)}
            onclick={() => choose(it)}
          >
            <span class="pal-ico">{glyph[it.kind]}</span>
            <span class="pal-main">
              <b>{it.label}</b>
              <small>{it.sub}</small>
            </span>
            {#if it.kind === "container"}
              <span class="badge {it.state}" style="margin-right:6px"><span class="dot"></span>{it.state}</span>
              {#if it.state === "running"}
                <span class="pal-act" role="button" tabindex="-1" onclick={(e) => act(it, "restart", e)}>Restart</span>
                <span class="pal-act danger" role="button" tabindex="-1" onclick={(e) => act(it, "stop", e)}>Stop</span>
              {:else}
                <span class="pal-act primary" role="button" tabindex="-1" onclick={(e) => act(it, "start", e)}>Start</span>
              {/if}
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .pal-overlay {
    align-items: flex-start;
    padding-top: 12vh;
  }
  .palette {
    width: min(680px, 92vw);
    background: var(--window);
    border: 1px solid var(--border);
    border-radius: 16px;
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .pal-input {
    width: 100%;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    padding: 16px 18px;
    font-size: 15px;
    color: var(--text);
    outline: none;
    border-radius: 0;
  }
  .pal-list {
    max-height: 52vh;
    overflow-y: auto;
    padding: 6px;
  }
  .pal-empty {
    padding: 26px;
    text-align: center;
    color: var(--text-faint);
  }
  .pal-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    text-align: left;
    padding: 9px 12px;
    border-radius: 10px;
    color: var(--text);
  }
  .pal-item.on {
    background: var(--accent-soft);
  }
  .pal-ico {
    width: 20px;
    text-align: center;
    color: var(--text-faint);
  }
  .pal-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .pal-main b {
    font-weight: 600;
  }
  .pal-main small {
    color: var(--text-faint);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pal-act {
    padding: 4px 9px;
    border-radius: 7px;
    font-size: 12px;
    font-weight: 600;
    border: 1px solid var(--border);
    color: var(--text-dim);
  }
  .pal-act:hover {
    background: var(--row-hover);
  }
  .pal-act.primary:hover {
    color: #fff;
    background: var(--green);
    border-color: var(--green);
  }
  .pal-act.danger:hover {
    color: #fff;
    background: var(--red);
    border-color: var(--red);
  }
</style>
