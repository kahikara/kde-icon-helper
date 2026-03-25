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

        <div class="inspectorInsightGrid">
          <div class="mainCard inspectorInsightCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Guidance</strong>
              <span class="mainMetaChip">{insight.launcherKindLabel}</span>
            </div>

            <div class="inspectorInsightTitle">{insight.issueTitle}</div>
            <div class="inspectorInsightText">{insight.issueSummary}</div>

            <div class="inspectorRecommendationBlock">
              <div class="inspectorRecommendationLabel">Recommended next step</div>
              <div class="inspectorRecommendationTitle">{insight.recommendationTitle}</div>
              <div class="inspectorRecommendationText">{insight.recommendationReason}</div>

              {#if insight.recommendedAction && canRunEntryAction(insight.recommendedAction)}
                <button
                  type="button"
                  class="inspectorRecommendedButton primary"
                  on:click={() => runEntryAction(insight.recommendedAction!)}
                  disabled={busy}
                >
                  {insight.recommendedActionLabel}
                </button>
              {/if}
            </div>
          </div>

          <div class="mainCard inspectorResolutionCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Resolution</strong>
            </div>

            <div class="inspectorResolutionGrid">
              <div class="resolutionFact">
                <span class="resolutionFactKey">Icon source</span>
                <span class="resolutionFactValue">{insight.iconSourceLabel}</span>
                <span class="resolutionFactText">{insight.iconSourceDetail}</span>
              </div>

              <div class="resolutionFact">
                <span class="resolutionFactKey">Launcher type</span>
                <span class="resolutionFactValue">{insight.launcherKindLabel}</span>
                <span class="resolutionFactText">{insight.launcherKindDetail}</span>
              </div>

              <div class="resolutionFact">
                <span class="resolutionFactKey">Target state</span>
                <span class="resolutionFactValue">{insight.targetStateLabel}</span>
                <span class="resolutionFactText">{insight.targetStateDetail}</span>
              </div>

              <div class="resolutionFact">
                <span class="resolutionFactKey">{insight.currentValueLabel}</span>
                <span class="resolutionFactValue code">{insight.currentValueDetail}</span>
                <span class="resolutionFactText">
                  Current launcher side value before any manual or automatic change.
                </span>
              </div>
            </div>
          </div>
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

            <div class="inspectorPreviewNote">
              This area shows the currently active icon source. It is the right anchor point for a later
              icon variants workflow.
            </div>
          </div>

          <div class="mainCard inspectorActionsCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Actions</strong>

              {#if insight.recommendedAction && canRunEntryAction(insight.recommendedAction)}
                <span class="mainMetaChip">Recommended</span>
              {/if}
            </div>

            <div class="inspectorActionHint">
              {#if insight.recommendedAction && canRunEntryAction(insight.recommendedAction)}
                Best next step for this state: {insight.recommendedActionLabel}.
              {:else}
                No automatic follow up is strongly recommended right now. Manual review is likely the better path.
              {/if}
            </div>

            <div class="inspectorActionStack">
              {#each entryActionItems as action}
                <button
                  type="button"
                  class="inspectorActionButton"
                  class:primary={!!action.primary}
                  class:recommended={action.id === insight.recommendedAction}
                  on:click={() => runEntryAction(action.id)}
                  disabled={busy || !canRunEntryAction(action.id)}
                >
                  {action.label}
                </button>
              {/each}
            </div>

            <div class="mainMiniFacts">
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

        <div class="inspectorBottomGrid">
          <div class="mainCard">
            <div class="mainSectionHeader">
              <strong class="mainSectionTitle">Launcher file</strong>
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
              <strong class="mainSectionTitle">Icon details</strong>
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
        <span>Select a launcher from the list to review its icon source, state and next best fix.</span>
      </div>
    </div>
  {/if}
</section>
