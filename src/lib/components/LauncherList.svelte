<script lang="ts">
  import type { LauncherEntry } from '$lib/types';

  export let filteredEntries: LauncherEntry[] = [];
  export let selected: LauncherEntry | null = null;
  export let listIconUrl: (entry: LauncherEntry) => string | null;
  export let rowGlyph: (entry: LauncherEntry) => string;
  export let statusClass: (status?: string | null) => string;
  export let statusText: (status?: string | null) => string;
  export let onSelect: (entry: LauncherEntry) => void;
  export let onContextMenu: (event: MouseEvent, entry: LauncherEntry) => void;

  function isSelectedEntry(entry: LauncherEntry) {
    return selected?.path === entry.path;
  }
</script>

<aside class="panel listPanel">
  <div class="panelHeader">
    <div class="panelTitle">Items</div>
  </div>

  <div class="listScroll">
    {#if filteredEntries.length === 0}
      <div class="empty">
        <strong>No items found</strong>
        <span>Try a different search or filter.</span>
      </div>
    {/if}

    {#each filteredEntries as entry}
      {@const rowIcon = listIconUrl(entry)}
      <button
        type="button"
        data-item-path={entry.path}
        class:selected={isSelectedEntry(entry)}
        class="itemCard"
        on:click={() => onSelect(entry)}
        on:contextmenu={(event) => onContextMenu(event, entry)}
      >
        <div class="itemIcon">
          {#if rowIcon}
            <img src={rowIcon} alt={`Icon for ${entry.name}`} />
          {:else}
            <span>{rowGlyph(entry)}</span>
          {/if}
        </div>

        <div class="itemName" title={entry.name}>{entry.name}</div>

        <div class="itemStatus">
          <span class={statusClass(entry.status)}>{statusText(entry.status)}</span>
        </div>
      </button>
    {/each}
  </div>
</aside>
