<script lang="ts">
  import type { LauncherEntry } from '$lib/types';
  import type { ContextAction, EntryActionItem } from '$lib/launcher-ui';
  import { deriveInspectorInsight } from '$lib/inspector-insights';

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

  $: insight = deriveInspectorInsight(selected);
  $: recommendedAction = insight.recommendedAction;
  $: recommendedActionAvailable =
    !!recommendedAction && canRunEntryAction(recommendedAction);
  $: isProblemState = !!selected && selected.status !== 'ok';
</script>

<section class="panel inspectorPanel">
  <div class="panelHeader">
    <div class="panelTitleWrap">
      <div class="panelTitle">Details</div>
    </div>

    {#if selected}
      <div class={`panelMetaChip inspectorStatusChip ${statusClass(selected.status)}`}>
        {statusText(selected.status)}
      </div>
    {/if}
  </div>

  {#if selected}
    <div class="inspectorScroll">
      <div class="mainSectionStack">
        <div class="mainCard inspectorSummaryCard">
          <div class="inspectorSummaryTop">
            <div class="inspectorSummaryTextWrap">
              <div class="inspectorSummaryName">{selected.name}</div>
              <div class="inspectorSummaryMessage">
                {insight.issueSummary}
              </div>
            </div>

            <div class="inspectorSummaryMeta">
              <span class="mainMetaChip">{busy ? 'Busy' : insight.iconSourceLabel}</span>
            </div>
          </div>

          {#if isProblemState}
            <div class="inspectorSummaryRecommendation">
              <div class="inspectorSummaryRecommendationText">
                <span class="inspectorSummaryRecommendationLabel">Next step</span>
                <span class="inspectorSummaryRecommendationTitle">{insight.recommendationTitle}</span>
                <span class="inspectorSummaryRecommendationReason">{insight.recommendationReason}</span>
              </div>

              {#if recommendedActionAvailable}
                <button
                  type="button"
                  class="inspectorRecommendedButton primary"
                  on:click={() => runEntryAction(recommendedAction!)}
                  disabled={busy}
                >
                  {insight.recommendedActionLabel}
                </button>
              {/if}
            </div>
          {/if}
        </div>

        <div class="inspectorTopGrid">
          <div class="mainCard inspectorPreviewCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Current icon</strong>
              <span class="mainMetaChip">{insight.iconSourceLabel}</span>
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
                  <span>The icon name exists, but no preview file is available yet.</span>
                </div>
              {:else}
                <div class="fallback">
                  <div class="fallbackGlyph">{previewFallbackGlyph(selected)}</div>
                  <strong>No preview available</strong>
                  <span>The current icon is missing, broken, or not previewable right now.</span>
                </div>
              {/if}
            </div>

            {#if isProblemState}
              <div class="inspectorPreviewNote">
                {insight.iconSourceDetail}
              </div>
            {/if}
          </div>

          <div class="mainCard inspectorActionsCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Actions</strong>

              {#if recommendedActionAvailable}
                <span class="mainMetaChip">Recommended</span>
              {/if}
            </div>

            <div class="inspectorActionLead">
              {#if recommendedActionAvailable}
                Best next step: {insight.recommendedActionLabel}
              {:else if isProblemState}
                Manual review is likely better than an automatic action here.
              {:else}
                No action needed right now.
              {/if}
            </div>

            <div class="inspectorActionStack">
              {#each entryActionItems as action}
                <button
                  type="button"
                  class="inspectorActionButton"
                  class:primary={!!action.primary}
                  class:recommended={action.id === recommendedAction}
                  on:click={() => runEntryAction(action.id)}
                  disabled={busy || !canRunEntryAction(action.id)}
                >
                  {action.label}
                </button>
              {/each}
            </div>

            <div class="mainMiniFacts">
              <div class="miniFact">
                <span class="miniFactKey">Icon source</span>
                <span class="miniFactValue">{insight.iconSourceLabel}</span>
              </div>

              <div class="miniFact">
                <span class="miniFactKey">Preview state</span>
                <span class="miniFactValue">{previewState}</span>
              </div>

              <div class="miniFact">
                <span class="miniFactKey">Restore support</span>
                <span class="miniFactValue">
                  {selected.canRestoreDefaultIcon ? 'Available' : 'Not available'}
                </span>
              </div>
            </div>
          </div>
        </div>

        <details class="mainCard inspectorAdvancedCard" open={isProblemState}>
          <summary class="inspectorAdvancedSummary">
            <span class="inspectorAdvancedTitle">Advanced details</span>
            <span class="mainMetaChip">{isProblemState ? 'Open for issues' : 'Optional'}</span>
          </summary>

          <div class="inspectorAdvancedBody">
            <div class="inspectorAdvancedGrid">
              <div class="inspectorAdvancedSection">
                <div class="inspectorAdvancedSectionTitle">Launcher</div>

                <div class="facts">
                  <div class="factKey">Desktop item</div>
                  <div class="factValue code">{selected.path}</div>

                  <div class="factKey">Target EXE</div>
                  <div class="factValue code">{selected.targetPath ?? 'None'}</div>

                  <div class="factKey">Target name</div>
                  <div class="factValue">{selectedExecName}</div>

                  <div class="factKey">Launcher type</div>
                  <div class="factValue">{insight.launcherKindLabel}</div>
                </div>
              </div>

              <div class="inspectorAdvancedSection">
                <div class="inspectorAdvancedSectionTitle">Icon</div>

                <div class="facts">
                  <div class="factKey">Icon value</div>
                  <div class="factValue code">{selected.icon ?? 'None'}</div>

                  <div class="factKey">Resolved icon</div>
                  <div class="factValue code">{selected.resolvedIconPath ?? 'None'}</div>

                  <div class="factKey">Target state</div>
                  <div class="factValue">{insight.targetStateLabel}</div>

                  <div class="factKey">Message</div>
                  <div class="factValue">{selected.message ?? 'No message available.'}</div>
                </div>
              </div>
            </div>
          </div>
        </details>
      </div>
    </div>
  {:else}
    <div class="mainEmptyCard">
      <div class="empty compact">
        <strong>No item selected</strong>
        <span>Select a launcher from the list to review its icon and available actions.</span>
      </div>
    </div>
  {/if}
</section>
