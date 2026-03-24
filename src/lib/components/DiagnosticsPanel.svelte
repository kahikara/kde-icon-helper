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
      {#if diagnostics}
        <div class="summaryGrid">
          <div class="summaryCard">
            <div class="summaryLabel">Desktop dir</div>
            <div class="summaryValue code">{diagnostics.desktopDir}</div>
          </div>

          <div
            class="summaryCard"
            class:issueCard={!diagnostics.desktopDirExists}
          >
            <div class="summaryLabel">Desktop dir exists</div>
            <div
              class="summaryValue"
              class:issueValue={!diagnostics.desktopDirExists}
            >
              {diagnostics.desktopDirExists ? 'Yes' : 'No'}
            </div>
          </div>

          <div class="summaryCard">
            <div class="summaryLabel">Found tools</div>
            <div class="summaryValue">{foundCount}</div>
          </div>

          <div class="summaryCard" class:issueCard={missingCount > 0}>
            <div class="summaryLabel">Missing tools</div>
            <div class="summaryValue" class:issueValue={missingCount > 0}>{missingCount}</div>
          </div>
        </div>

        <div class="contentGrid">
          {#each diagnostics.tools as tool}
            <div class="contentCard">
              <div class="cardTopRow">
                <strong>{tool.name}</strong>
                <span class="toolState" class:good={tool.found} class:bad={!tool.found}>
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

              <div class="dataRow">
                <span class="dataKey">Used for</span>
                <span class="dataValue">{tool.requiredFor.join(', ')}</span>
              </div>

              {#if tool.note}
                <div class="dataRow">
                  <span class="dataKey">Note</span>
                  <span class="dataValue">{tool.note}</span>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <div class="empty compact">
          <strong>No diagnostics loaded yet</strong>
          <span>Use refresh to probe the current runtime environment.</span>
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
    gap: var(--utility-gap, 10px);
  }

  .summaryGrid {
    display: grid;
    grid-template-columns: repeat(4, minmax(170px, 1fr));
    gap: var(--utility-gap, 10px);
  }

  .summaryCard,
  .contentCard {
    border: var(--utility-card-border, 1px solid rgba(255, 255, 255, 0.08));
    border-radius: var(--utility-card-radius, 12px);
    background: var(--utility-card-bg, rgba(255, 255, 255, 0.02));
    box-shadow: var(--utility-card-shadow, none);
    padding: var(--utility-card-padding, 10px 12px);
  }

  .summaryCard {
    min-height: 104px;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .summaryLabel {
    font-size: 0.74rem;
    opacity: 0.72;
    margin-bottom: 6px;
  }

  .summaryValue {
    font-size: 0.92rem;
    font-weight: 600;
    line-height: 1.35;
    word-break: break-word;
  }

  .issueCard {
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.024)),
      rgba(255, 255, 255, 0.02);
  }

  .issueValue {
    opacity: 0.96;
  }

  .contentGrid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--utility-gap, 10px);
  }

  .cardTopRow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 10px;
    color: var(--utility-strong-text, inherit);
  }

  .toolState {
    font-size: 0.78rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .dataRow {
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 10px;
    margin-top: 6px;
  }

  .dataKey {
    font-size: 0.76rem;
    opacity: 0.72;
  }

  .dataValue {
    font-size: 0.84rem;
    line-height: 1.4;
    word-break: break-word;
    min-width: 0;
  }

  .code {
    font-family: monospace;
    font-size: 0.8rem;
  }

  .good {
    opacity: 0.9;
  }

  .bad {
    opacity: 0.95;
  }

  @media (max-width: 1100px) {
    .summaryGrid,
    .contentGrid {
      grid-template-columns: 1fr;
    }

    .summaryCard {
      min-height: auto;
    }
  }
</style>
