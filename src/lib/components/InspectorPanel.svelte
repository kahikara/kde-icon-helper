<script lang="ts">
  import type { LauncherEntry } from '$lib/types';
  import type { ContextAction, EntryActionItem } from '$lib/launcher-ui';

  export let selected: LauncherEntry | null = null;
  export let busy = false;
  export let selectedIconUrl: string | null = null;
  export let iconLoadFailed = false;
  export let selectedHasThemeIcon = false;
  export let selectedExecName = 'None';
  export let entryActionItems: EntryActionItem[] = [];
  export let statusClass: (status?: string | null) => string;
  export let statusText: (status?: string | null) => string;
  export let previewFallbackGlyph: (entry: LauncherEntry) => string;
  export let canRunEntryAction: (action: ContextAction) => boolean;
  export let runEntryAction: (action: ContextAction) => Promise<void> | void;
  export let onPreviewError: () => void;
</script>

<section class="panel inspectorPanel">
  <div class="panelHeader">
    <div class="panelTitle">Inspector</div>
  </div>

  {#if selected}
    <div class="inspectorScroll">
      <div class="field">
        <div class="label">Name</div>
        <div class="value">{selected.name}</div>
      </div>

      <div class="field">
        <div class="label">Status</div>
        <div class="value">
          <span class={statusClass(selected.status)}>{statusText(selected.status)}</span>
        </div>
      </div>

      <div class="field previewField">
        <div class="label">Preview</div>
        <div class="preview">
          {#if selectedIconUrl && !iconLoadFailed}
            <img
              src={selectedIconUrl}
              alt={`Current icon for ${selected.name}`}
              on:error={onPreviewError}
            />
          {:else if selectedHasThemeIcon}
            <div class="fallback">
              <div class="fallbackGlyph">☆</div>
              <strong>Theme icon</strong>
              <span>The icon name was found, but no preview file could be loaded yet.</span>
            </div>
          {:else}
            <div class="fallback">
              <div class="fallbackGlyph">{previewFallbackGlyph(selected)}</div>
              <strong>No preview available</strong>
              <span>The current icon is missing, broken, or not previewable yet.</span>
            </div>
          {/if}
        </div>
      </div>

      <div class="field">
        <div class="label">Actions</div>
        <div class="inspectorActions">
          {#each entryActionItems as action}
            <button
              type="button"
              class:primary={!!action.primary}
              on:click={() => runEntryAction(action.id)}
              disabled={busy || !canRunEntryAction(action.id)}
            >
              {action.label}
            </button>
          {/each}
        </div>
      </div>

      <div class="facts">
        <div class="factKey">Desktop item</div>
        <div class="factValue code">{selected.path}</div>

        <div class="factKey">Target EXE</div>
        <div class="factValue code">{selected.targetPath ?? 'None'}</div>

        <div class="factKey">Icon value</div>
        <div class="factValue code">{selected.icon ?? 'None'}</div>

        <div class="factKey">Resolved icon</div>
        <div class="factValue code">{selected.resolvedIconPath ?? 'None'}</div>

        <div class="factKey">Target name</div>
        <div class="factValue">{selectedExecName}</div>

        <div class="factKey">Can restore default</div>
        <div class="factValue">{selected.canRestoreDefaultIcon ? 'Yes' : 'No'}</div>

        <div class="factKey">Message</div>
        <div class="factValue">{selected.message ?? 'No message available.'}</div>
      </div>
    </div>
  {:else}
    <div class="empty">
      <strong>No item selected</strong>
      <span>Pick one from the list to inspect it.</span>
    </div>
  {/if}
</section>
