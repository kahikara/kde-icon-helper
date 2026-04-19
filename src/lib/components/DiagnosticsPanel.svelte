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
          <span class="panelSubtle">{missingCount === 0 ? 'All key tools found' : `${missingCount} missing`}</span>
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
        <div class="quickStats">
          <div class="statCard" class:alertCard={!diagnostics.desktopDirExists}>
            <div class="statLabel">Desktop dir</div>
            <div class="statValue">{diagnostics.desktopDirExists ? 'Ready' : 'Check'}</div>
            <div class="statSubtle code">{diagnostics.desktopDir}</div>
          </div>

          <div class="statCard">
            <div class="statLabel">Found tools</div>
            <div class="statValue">{foundCount}</div>
            <div class="statSubtle">Available now</div>
          </div>

          <div class="statCard" class:alertCard={missingCount > 0}>
            <div class="statLabel">Missing tools</div>
            <div class="statValue">{missingCount}</div>
            <div class="statSubtle">{missingCount > 0 ? 'Needs review' : 'All key tools found'}</div>
          </div>
        </div>

        <div class="contentGrid">
          <div class="contentCard">
            <div class="sectionHeader">
              <strong class="sectionTitle">Environment</strong>
              <span class="sectionMeta">{foundCount} found</span>
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

          <div class="contentCard">
            <div class="sectionHeader">
              <strong class="sectionTitle">Tools</strong>
              <span class="sectionMeta">{diagnostics.tools.length} checked</span>
            </div>

            <div class="toolList">
              {#each diagnostics.tools as tool}
                <div class="toolRow" class:alertCard={!tool.found}>
                  <div class="toolMain">
                    <div class="toolName">{tool.name}</div>
                    <div class="toolMeta">{tool.version ?? 'Version unknown'}</div>
                  </div>

                  <div class="toolSide">
                    <span class="toolStateChip" class:toolStateChipAlert={!tool.found}>
                      {tool.found ? 'Found' : 'Missing'}
                    </span>
                  </div>

                  <div class="toolDetails">
                    <div class="toolPath code">{tool.path ?? 'Not found'}</div>

                    {#if tool.requiredFor.length > 0}
                      <div class="chipWrap">
                        {#each tool.requiredFor as item}
                          <span class="softChip">{item}</span>
                        {/each}
                      </div>
                    {/if}

                    {#if tool.note}
                      <div class="toolNote">{tool.note}</div>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </div>
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
    gap: var(--utility-gap, 8px);
  }

  .statCard,
  .contentCard {
    border: var(--utility-card-border, 1px solid rgba(255, 255, 255, 0.08));
    border-radius: var(--utility-card-radius, 11px);
    background: var(--utility-card-bg, rgba(255, 255, 255, 0.02));
    box-shadow: var(--utility-card-shadow, none);
    padding: var(--utility-card-padding, 9px 11px);
    min-width: 0;
  }

  .quickStats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--utility-gap, 8px);
  }

  .statCard {
    display: grid;
    gap: 4px;
  }

  .statLabel {
    font-size: 0.73rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .statValue {
    font-size: 0.96rem;
    font-weight: 700;
    line-height: 1.25;
  }

  .statSubtle {
    font-size: 0.76rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .alertCard {
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.022)),
      rgba(255, 255, 255, 0.02);
  }

  .contentGrid {
    display: grid;
    grid-template-columns: minmax(280px, 0.9fr) minmax(0, 1.1fr);
    gap: var(--utility-gap, 8px);
    align-items: start;
  }

  .sectionHeader {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .sectionTitle {
    font-size: 0.81rem;
    font-weight: 700;
    line-height: 1.2;
  }

  .sectionMeta,
  .toolStateChip {
    font-size: 0.74rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.76));
    padding: 2px 7px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.04);
    white-space: nowrap;
  }

  .toolStateChipAlert {
    color: var(--utility-strong-text, rgba(255, 255, 255, 0.95));
    background: rgba(255, 255, 255, 0.07);
  }

  .dataRow {
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 10px;
    margin-top: 7px;
  }

  .dataKey {
    font-size: 0.75rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .dataValue {
    font-size: 0.82rem;
    line-height: 1.36;
    word-break: break-word;
    min-width: 0;
  }

  .toolList {
    display: grid;
    gap: 7px;
  }

  .toolRow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    align-items: start;
    padding: 8px 9px;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.04);
    background: rgba(255, 255, 255, 0.018);
  }

  .toolMain {
    min-width: 0;
    display: grid;
    gap: 2px;
  }

  .toolName {
    font-size: 0.81rem;
    font-weight: 600;
    line-height: 1.22;
  }

  .toolMeta {
    font-size: 0.74rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .toolSide {
    display: flex;
    align-items: center;
  }

  .toolDetails {
    grid-column: 1 / -1;
    display: grid;
    gap: 6px;
  }

  .toolPath {
    font-family: monospace;
    font-size: 0.79rem;
    line-height: 1.34;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
    word-break: break-word;
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
    min-height: 21px;
    padding: 0 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.04);
    font-size: 0.73rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.76));
    line-height: 1.2;
  }

  .toolNote {
    font-size: 0.77rem;
    line-height: 1.34;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .code {
    font-family: monospace;
    font-size: 0.79rem;
  }

  .emptyCard {
    padding: 13px;
  }

  @media (max-width: 1100px) {
    .quickStats,
    .contentGrid {
      grid-template-columns: 1fr;
    }
  }
</style>
