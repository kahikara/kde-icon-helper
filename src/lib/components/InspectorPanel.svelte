<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { ContextAction, EntryActionItem } from '$lib/launcher-ui';
  import { deriveInspectorInsight } from '$lib/inspector-insights';
  import type { IconVariant, LauncherEntry } from '$lib/types';

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
  export let onApplyIconVariant: (sourceIconPath: string) => Promise<void> | void;
  export let onPreviewError: () => void;

  let technicalOpen = true;
  let variantsOpen = false;
  let iconVariants: IconVariant[] = [];
  let iconVariantsBusy = false;
  let applyingVariantPath: string | null = null;
  let iconVariantPreviewUrls: Record<string, string> = {};
  let variantsLoadedForPath = '';
  let variantRequestToken = 0;

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
  $: nextStepText = recommendedActionAvailable
    ? insight.recommendedActionLabel
    : isProblemState
      ? 'Manual review'
      : 'None needed';

  $: iconVariantOptions = iconVariants.filter((variant) => !variant.isCurrent);
  $: iconVariantCount = iconVariantOptions.length;
  $: iconVariantButtonLabel = iconVariantsBusy
    ? 'Searching…'
    : iconVariantCount === 1
      ? '1 other icon'
      : `${iconVariantCount} other icons`;

  $: showTargetContext = !!selected?.targetPath || isProblemState;

  $: {
    const path = selected?.path ?? '';
    if (path !== variantsLoadedForPath) {
      variantsLoadedForPath = path;
      variantsOpen = false;
      iconVariants = [];
      iconVariantsBusy = false;
      applyingVariantPath = null;
      iconVariantPreviewUrls = {};

      if (path) {
        void refreshIconVariants(path);
      }
    }
  }

  function variantPreviewUrl(path: string): string | null {
    const value = iconVariantPreviewUrls[path];
    return value && value.length > 0 ? value : null;
  }

  async function loadVariantPreview(path: string, requestToken: number) {
    if (iconVariantPreviewUrls[path]) {
      return;
    }

    try {
      const result = await invoke<string | null>('load_icon_preview', { path });
      if (requestToken !== variantRequestToken) {
        return;
      }

      if (result) {
        iconVariantPreviewUrls = {
          ...iconVariantPreviewUrls,
          [path]: result
        };
      }
    } catch {
      // optional preview only
    }
  }

  async function refreshIconVariants(path: string) {
    const requestToken = ++variantRequestToken;
    iconVariantsBusy = true;

    try {
      const result = await invoke<IconVariant[]>('list_icon_variants', { path });

      if (requestToken !== variantRequestToken || (selected?.path ?? '') !== path) {
        return;
      }

      iconVariants = result;

      for (const variant of result.filter((value) => !value.isCurrent).slice(0, 8)) {
        void loadVariantPreview(variant.path, requestToken);
      }
    } catch {
      if (requestToken === variantRequestToken) {
        iconVariants = [];
      }
    } finally {
      if (requestToken === variantRequestToken && (selected?.path ?? '') === path) {
        iconVariantsBusy = false;
      }
    }
  }

  async function applyIconVariant(variant: IconVariant) {
    const selectedPath = selected?.path;
    if (!selectedPath || busy || applyingVariantPath || variant.isCurrent) {
      return;
    }

    applyingVariantPath = variant.path;

    try {
      await onApplyIconVariant(variant.path);
      variantsOpen = false;
      variantsLoadedForPath = '';
      await refreshIconVariants(selectedPath);
    } finally {
      applyingVariantPath = null;
    }
  }
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
            </div>

            <div class="inspectorPreviewWrap">
              <div class="preview">
                {#if selectedIconUrl && !iconLoadFailed}
                  <img
                    src={selectedIconUrl}
                    alt={`Current icon for ${selected.name}`}
                    on:error={onPreviewError}
                  />
                {:else}
                  <div class="fallback">
                    <div class="fallbackGlyph">{previewFallbackGlyph(selected)}</div>
                    <strong>No preview available</strong>
                    <span>
                      {selectedHasThemeIcon
                        ? 'The icon is resolved by name, but no preview file is available right now.'
                        : 'The current icon is missing, broken, or not previewable right now.'}
                    </span>
                  </div>
                {/if}
              </div>

              {#if iconVariantsBusy || iconVariantCount > 0}
                <div class="iconVariantBar">
                  <div class="iconVariantBarText">
                    {iconVariantsBusy
                      ? 'Searching for additional icons…'
                      : iconVariantCount === 1
                        ? '1 additional icon found'
                        : `${iconVariantCount} additional icons found`}
                  </div>

                  <button
                    type="button"
                    class="ghost inspectorHeaderButton"
                    on:click={() => (variantsOpen = !variantsOpen)}
                  >
                    {variantsOpen ? 'Hide icons' : 'Browse icons'}
                  </button>
                </div>
              {/if}

              {#if variantsOpen && iconVariantCount > 0}
                <div class="iconVariantOverlay">
                  <div class="iconVariantOverlayHeader">
                    <div class="iconVariantOverlayTitle">Other icons</div>

                    <button
                      type="button"
                      class="ghost inspectorHeaderButton"
                      on:click={() => (variantsOpen = false)}
                    >
                      Close
                    </button>
                  </div>

                  <div class="iconVariantList">
                    {#each iconVariantOptions as variant}
                      <div class="iconVariantRow">
                        <div class="iconVariantPreview">
                          {#if variantPreviewUrl(variant.path)}
                            <img src={variantPreviewUrl(variant.path)!} alt={variant.label} />
                          {:else}
                            <div class="iconVariantPreviewGlyph">☆</div>
                          {/if}
                        </div>

                        <div class="iconVariantMeta">
                          <div class="iconVariantName">{variant.label}</div>
                          <div class="iconVariantSource">{variant.source}</div>
                        </div>

                        <button
                          type="button"
                          class="ghost iconVariantButton"
                          on:click={() => applyIconVariant(variant)}
                          disabled={busy || applyingVariantPath === variant.path}
                        >
                          {applyingVariantPath === variant.path ? 'Applying…' : 'Use'}
                        </button>
                      </div>
                    {/each}
                  </div>
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

        <div class="mainCard inspectorAdvancedCard" class:isCollapsed={!technicalOpen}>
          <div class="mainSectionHeader inspectorAdvancedHeader">
            <strong class="mainSectionTitle">Technical details</strong>

            <button
              type="button"
              class="ghost inspectorToggleButton"
              aria-expanded={technicalOpen}
              on:click={() => (technicalOpen = !technicalOpen)}
            >
              {technicalOpen ? 'Collapse' : 'Expand'}
            </button>
          </div>

          {#if technicalOpen}
            <div class="inspectorAdvancedGrid">
              <div class="inspectorAdvancedSection">
                <div class="inspectorAdvancedSectionTitle">Launcher</div>

                <div class="facts">
                  <div class="factKey">Desktop item</div>
                  <div class="factValue code">{selected.path}</div>

                  <div class="factKey">Launcher type</div>
                  <div class="factValue">{insight.launcherKindLabel}</div>

                  <div class="factKey">Target EXE</div>
                  <div class="factValue code">{selected.targetPath ?? 'None'}</div>

                  <div class="factKey">Target name</div>
                  <div class="factValue">{selectedExecName}</div>
                </div>
              </div>

              <div class="inspectorAdvancedSection">
                <div class="inspectorAdvancedSectionTitle">Icon</div>

                <div class="facts">
                  <div class="factKey">Icon value</div>
                  <div class="factValue code">{selected.icon ?? 'None'}</div>

                  <div class="factKey">Resolved icon</div>
                  <div class="factValue code">{selected.resolvedIconPath ?? 'None'}</div>

                  <div class="factKey">Other icons</div>
                  <div class="factValue">{iconVariantCount > 0 ? iconVariantButtonLabel : 'None'}</div>

                  <div class="factKey">Restore support</div>
                  <div class="factValue">
                    {selected.canRestoreDefaultIcon ? 'Available' : 'Not available'}
                  </div>
                </div>
              </div>
            </div>

            <div class="inspectorContextGrid">
              <div class="inspectorContextCard">
                <div class="inspectorContextTitle">Source detail</div>
                <div class="inspectorContextText">{insight.iconSourceDetail}</div>
              </div>

              {#if showTargetContext}
                <div class="inspectorContextCard">
                  <div class="inspectorContextTitle">Target state</div>
                  <div class="inspectorContextText">
                    {selected.targetPath ?? insight.targetStateDetail}
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>
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
