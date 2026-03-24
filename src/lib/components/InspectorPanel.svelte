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

  $: previewState =
    selectedIconUrl && !iconLoadFailed
      ? 'Loaded'
      : selectedHasThemeIcon
        ? 'Theme'
        : 'Fallback';
</script>

<section class="panel inspectorPanel">
  <div class="panelHeader">
    <div class="panelTitleWrap">
      <div class="panelTitle">Details</div>
    </div>

    {#if selected}
      <div class="panelMetaChip">{statusText(selected.status)}</div>
    {/if}
  </div>

  {#if selected}
    <div class="inspectorScroll">
      <div class="mainSectionStack">
        <div class="mainCard inspectorHeroCard">
          <div class="inspectorHeroTop">
            <div class="inspectorHeroText">
              <div class="inspectorHeroName">{selected.name}</div>
            </div>

            <div class="inspectorHeroBadges">
              <span class={statusClass(selected.status)}>{statusText(selected.status)}</span>
              <span class="mainMetaChip">{busy ? 'Busy' : 'Ready'}</span>
            </div>
          </div>

          <div class="inspectorHeroMessage">
            {selected.message ?? 'No message available.'}
          </div>
        </div>

        <div class="inspectorTopGrid">
          <div class="mainCard inspectorPreviewCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Preview</strong>
            </div>

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

          <div class="mainCard inspectorActionsCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Actions</strong>
            </div>

            <div class="inspectorActionStack">
              {#each entryActionItems as action}
                <button
                  type="button"
                  class="inspectorActionButton"
                  class:primary={!!action.primary}
                  on:click={() => runEntryAction(action.id)}
                  disabled={busy || !canRunEntryAction(action.id)}
                >
                  {action.label}
                </button>
              {/each}
            </div>

            <div class="mainMiniFacts">
              <div class="miniFact">
                <span class="miniFactKey">Target name</span>
                <span class="miniFactValue">{selectedExecName}</span>
              </div>

              <div class="miniFact">
                <span class="miniFactKey">Can restore default</span>
                <span class="miniFactValue">{selected.canRestoreDefaultIcon ? 'Yes' : 'No'}</span>
              </div>
            </div>
          </div>
        </div>

        <div class="inspectorBottomGrid">
          <div class="mainCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Launcher</strong>
            </div>

            <div class="facts">
              <div class="factKey">Desktop item</div>
              <div class="factValue code">{selected.path}</div>

              <div class="factKey">Target EXE</div>
              <div class="factValue code">{selected.targetPath ?? 'None'}</div>

              <div class="factKey">Target name</div>
              <div class="factValue">{selectedExecName}</div>

              <div class="factKey">Can restore default</div>
              <div class="factValue">{selected.canRestoreDefaultIcon ? 'Yes' : 'No'}</div>
            </div>
          </div>

          <div class="mainCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Icon</strong>
            </div>

            <div class="facts">
              <div class="factKey">Icon value</div>
              <div class="factValue code">{selected.icon ?? 'None'}</div>

              <div class="factKey">Resolved icon</div>
              <div class="factValue code">{selected.resolvedIconPath ?? 'None'}</div>

              <div class="factKey">Preview state</div>
              <div class="factValue">{previewState}</div>

              <div class="factKey">Message</div>
              <div class="factValue">{selected.message ?? 'No message available.'}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  {:else}
    <div class="mainEmptyCard">
      <div class="empty compact">
        <strong>No item selected</strong>
        <span>Pick one from the list to inspect it.</span>
      </div>
    </div>
  {/if}
</section>
