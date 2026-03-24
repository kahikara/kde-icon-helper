<script lang="ts">
  import type { RuntimeDiagnostics } from '$lib/types';

  export let diagnostics: RuntimeDiagnostics | null = null;
  export let diagnosticsOpen = false;
  export let diagnosticsBusy = false;
  export let missingCount = 0;
  export let embedded = false;
  export let onToggle: () => void;
  export let onRefresh: () => Promise<void> | void;

  $: foundCount = diagnostics ? diagnostics.tools.filter((tool) => tool.found).length : 0;
  $: hasEnvironmentIssues = diagnostics ? !diagnostics.desktopDirExists || missingCount > 0 : missingCount > 0;
</script>

<section class="panel utilityPanel" class:embeddedPanel={embedded}>
  {#if !embedded}
    <div class="panelHeader logHeader">
      <div class="panelTitle">
        Diagnostics
        {#if diagnostics}
          <span class="panelSubtle">
            {missingCount === 0 ? 'All key tools found' : `${missingCount} missing`}
          </span>
        {/if}
      </div>

      <div class="panelActions">
        <button type="button" class="ghost" on:click={onRefresh} disabled={diagnosticsBusy}>
          {diagnosticsBusy ? 'Refreshing…' : 'Refresh'}
        </button>
        <button type="button" class="ghost" on:click={onToggle}>
          {diagnosticsOpen ? 'Hide' : 'Show'}
        </button>
      </div>
    </div>
  {/if}

  {#if embedded || diagnosticsOpen}
    <div class="panelBody">
      {#if embedded}
        <div class="introCard">
          <div class="introEyebrow">Diagnostics</div>
          <div class="introRow">
            <strong class="introTitle">Inspect the runtime environment</strong>
            <span class="introMeta">{missingCount} missing</span>
          </div>
          <div class="introText">
            Validate required tools and review the current desktop environment setup.
          </div>
        </div>
      {/if}

      {#if diagnostics}
        {#if hasEnvironmentIssues}
          <div class="bannerCard alertBanner">
            <div class="bannerTextWrap">
              <strong class="bannerTitle">Environment needs attention</strong>
              <span class="bannerText">
                Some required tools are missing or the desktop directory is not available.
              </span>
            </div>
            <span class="bannerMeta">Check setup</span>
          </div>
        {/if}

        <div class="summaryGrid">
          <div class="summaryCard">
            <div class="summaryTopRow">
              <div class="summaryLabel">Desktop dir</div>
              <span class="summaryTone">Path</span>
            </div>
            <div class="summaryValue code">{diagnostics.desktopDir}</div>
          </div>

          <div class="summaryCard" class:alertCard={!diagnostics.desktopDirExists}>
            <div class="summaryTopRow">
              <div class="summaryLabel">Desktop dir exists</div>
              <span class="summaryTone" class:summaryToneAlert={!diagnostics.desktopDirExists}>
                {diagnostics.desktopDirExists ? 'OK' : 'Check'}
              </span>
            </div>
            <div class="summaryValue">{diagnostics.desktopDirExists ? 'Yes' : 'No'}</div>
          </div>

          <div class="summaryCard">
            <div class="summaryTopRow">
              <div class="summaryLabel">Found tools</div>
              <span class="summaryTone">Ready</span>
            </div>
            <div class="summaryValue">{foundCount}</div>
          </div>

          <div class="summaryCard" class:alertCard={missingCount > 0}>
            <div class="summaryTopRow">
              <div class="summaryLabel">Missing tools</div>
              <span class="summaryTone" class:summaryToneAlert={missingCount > 0}>
                {missingCount > 0 ? 'Attention' : 'Clean'}
              </span>
            </div>
            <div class="summaryValue">{missingCount}</div>
          </div>
        </div>

        <div class="contentGrid">
          <div class="contentCard spanTwo">
            <div class="sectionHeader">
              <strong class="sectionTitle">Environment overview</strong>
              <span class="sectionMeta">{foundCount} tool(s) found</span>
            </div>

            <div class="dataRow">
              <span class="dataKey">Desktop dir</span>
              <span class="dataValue code">{diagnostics.desktopDir}</span>
            </div>

            <div class="dataRow">
              <span class="dataKey">Exists</span>
              <span class="dataValue">{diagnostics.desktopDirExists ? 'Yes' : 'No'}</span>
            </div>
          </div>

          {#each diagnostics.tools as tool}
            <div class="contentCard toolCard" class:alertCard={!tool.found}>
              <div class="sectionHeader">
                <strong class="sectionTitle">{tool.name}</strong>
                <span class="sectionMeta toolStateChip" class:toolStateChipAlert={!tool.found}>
                  {tool.found ? 'Found' : 'Missing'}
                </span>
              </div>

              <div class="dataRow">
                <span class="dataKey">Path</span>
                <span class="dataValue code">{tool.path ?? 'Not found'}</span>
              </div>

              <div class="dataRow">
                <span class="dataKey">Version</span>
                <span class="dataValue">{tool.version ?? 'Unknown'}</span>
              </div>

              <div class="dataRow stackRow">
                <span class="dataKey">Used for</span>
                <div class="chipWrap">
                  {#each tool.requiredFor as item}
                    <span class="softChip">{item}</span>
                  {/each}
                </div>
              </div>

              {#if tool.note}
                <div class="dataRow stackRow">
                  <span class="dataKey">Note</span>
                  <span class="dataValue">{tool.note}</span>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <div class="contentCard emptyCard">
          <div class="empty compact">
            <strong>No diagnostics loaded yet</strong>
            <span>Use refresh to probe the current runtime environment.</span>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .embeddedPanel {
    padding: 0;
    background: transparent;
    box-shadow: none;
    border: 0;
  }

  .panelSubtle {
    font-size: 0.78rem;
    opacity: 0.78;
    margin-left: 8px;
  }

  .panelActions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .panelBody {
    display: flex;
    flex-direction: column;
    gap: var(--utility-gap, 9px);
  }

  .introCard,
  .summaryCard,
  .contentCard,
  .bannerCard {
    border: var(--utility-card-border, 1px solid rgba(255, 255, 255, 0.08));
    border-radius: var(--utility-card-radius, 11px);
    background: var(--utility-card-bg, rgba(255, 255, 255, 0.02));
    box-shadow: var(--utility-card-shadow, none);
    padding: var(--utility-card-padding, 9px 11px);
    min-width: 0;
  }

  .introEyebrow {
    font-size: 0.71rem;
    opacity: 0.68;
    margin-bottom: 4px;
  }

  .introRow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
  }

  .introTitle {
    font-size: 0.88rem;
    line-height: 1.2;
  }

  .introMeta,
  .sectionMeta,
  .summaryTone,
  .bannerMeta,
  .toolStateChip {
    font-size: 0.74rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.76));
    padding: 2px 7px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.04);
    white-space: nowrap;
  }

  .summaryToneAlert,
  .toolStateChipAlert {
    color: var(--utility-strong-text, rgba(255, 255, 255, 0.95));
    background: rgba(255, 255, 255, 0.07);
  }

  .introText,
  .bannerText {
    font-size: 0.8rem;
    line-height: 1.38;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .alertBanner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.05), rgba(255, 255, 255, 0.025)),
      rgba(255, 255, 255, 0.02);
  }

  .bannerTextWrap {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .bannerTitle {
    font-size: 0.83rem;
    line-height: 1.2;
  }

  .summaryGrid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--utility-gap, 9px);
  }

  .summaryCard {
    min-height: 104px;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .summaryTopRow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 7px;
  }

  .summaryLabel {
    font-size: 0.73rem;
    opacity: 0.72;
  }

  .summaryValue {
    font-size: 0.9rem;
    font-weight: 600;
    line-height: 1.32;
    word-break: break-word;
  }

  .alertCard {
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.05), rgba(255, 255, 255, 0.025)),
      rgba(255, 255, 255, 0.02);
  }

  .contentGrid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--utility-gap, 9px);
  }

  .spanTwo {
    grid-column: 1 / -1;
  }

  .toolCard {
    align-self: start;
  }

  .sectionHeader {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 9px;
  }

  .sectionTitle {
    font-size: 0.81rem;
    font-weight: 700;
    line-height: 1.2;
  }

  .dataRow {
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 10px;
    margin-top: 7px;
  }

  .stackRow {
    align-items: start;
  }

  .dataKey {
    font-size: 0.75rem;
    opacity: 0.72;
  }

  .dataValue {
    font-size: 0.82rem;
    line-height: 1.38;
    word-break: break-word;
    min-width: 0;
  }

  .chipWrap {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }

  .softChip {
    display: inline-flex;
    align-items: center;
    min-height: 22px;
    padding: 0 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.04);
    font-size: 0.74rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.76));
    line-height: 1.2;
  }

  .code {
    font-family: monospace;
    font-size: 0.79rem;
  }

  .emptyCard {
    padding: 13px;
  }

  @media (max-width: 1100px) {
    .summaryGrid,
    .contentGrid {
      grid-template-columns: 1fr;
    }

    .summaryCard {
      min-height: auto;
    }

    .spanTwo {
      grid-column: auto;
    }

    .alertBanner {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
