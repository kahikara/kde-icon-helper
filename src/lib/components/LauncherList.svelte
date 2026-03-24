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

  function entrySecondaryText(entry: LauncherEntry) {
    return entry.targetPath ?? entry.path;
  }
</script>

<aside class="panel listPanel">
  <div class="panelHeader">
    <div class="panelTitleWrap">
      <div class="panelTitle">Items</div>
      <div class="panelSubline">Desktop launchers and direct EXE links</div>
    </div>

    <div class="panelMetaChip">{filteredEntries.length}</div>
  </div>

  <div class="listScroll">
    {#if filteredEntries.length === 0}
      <div class="mainEmptyCard">
        <div class="empty compact">
          <strong>No items found</strong>
          <span>Try a different search or filter.</span>
        </div>
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

        <div class="itemBody">
          <div class="itemName" title={entry.name}>{entry.name}</div>
          <div class="itemMetaLine" title={entrySecondaryText(entry)}>
            {entrySecondaryText(entry)}
          </div>
        </div>

        <div class="itemStatus">
          <span class={statusClass(entry.status)}>{statusText(entry.status)}</span>
        </div>
      </button>
    {/each}
  </div>
</aside>
