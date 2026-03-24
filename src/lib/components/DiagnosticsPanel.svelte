<script lang="ts">
  import type { RuntimeDiagnostics } from '$lib/types';

  export let diagnostics: RuntimeDiagnostics | null = null;
  export let diagnosticsOpen = false;
  export let diagnosticsBusy = false;
  export let missingCount = 0;
  export let onToggle: () => void;
  export let onRefresh: () => Promise<void> | void;
</script>

<section class="panel diagnosticsPanel">
  <div class="panelHeader logHeader">
    <div class="panelTitle">
      Diagnostics
      {#if diagnostics}
        <span class="diagSummary">
          {missingCount === 0 ? 'All key tools found' : `${missingCount} missing`}
        </span>
      {/if}
    </div>

    <div class="diagActions">
      <button type="button" class="ghost" on:click={onRefresh} disabled={diagnosticsBusy}>
        {diagnosticsBusy ? 'Refreshing…' : 'Refresh'}
      </button>
      <button type="button" class="ghost" on:click={onToggle}>
        {diagnosticsOpen ? 'Hide' : 'Show'}
      </button>
    </div>
  </div>

  {#if diagnosticsOpen}
    <div class="diagScroll">
      {#if diagnostics}
        <div class="diagMeta">
          <div class="diagMetaRow">
            <strong>Desktop dir</strong>
            <span class:bad={!diagnostics.desktopDirExists}>{diagnostics.desktopDir}</span>
          </div>
          <div class="diagMetaRow">
            <strong>Desktop dir exists</strong>
            <span class:bad={!diagnostics.desktopDirExists}>
              {diagnostics.desktopDirExists ? 'Yes' : 'No'}
            </span>
          </div>
        </div>

        <div class="diagTable">
          {#each diagnostics.tools as tool}
            <div class="diagCard">
              <div class="diagTopRow">
                <strong>{tool.name}</strong>
                <span class:good={tool.found} class:bad={!tool.found}>
                  {tool.found ? 'Found' : 'Missing'}
                </span>
              </div>

              <div class="diagLine">
                <span class="diagKey">Path</span>
                <span class="diagValue code">{tool.path ?? 'Not found'}</span>
              </div>

              <div class="diagLine">
                <span class="diagKey">Version</span>
                <span class="diagValue">{tool.version ?? 'Unknown'}</span>
              </div>

              <div class="diagLine">
                <span class="diagKey">Used for</span>
                <span class="diagValue">{tool.requiredFor.join(', ')}</span>
              </div>

              {#if tool.note}
                <div class="diagLine">
                  <span class="diagKey">Note</span>
                  <span class="diagValue">{tool.note}</span>
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
