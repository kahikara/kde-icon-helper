<script lang="ts">
  import type { LauncherEntry } from '$lib/types';
  import type {
    ContextAction,
    ContextMenuMode,
    EntryActionItem,
    InputActionItem,
    InputContextAction
  } from '$lib/launcher-ui';

  export let open = false;
  export let mode: ContextMenuMode = 'entry';
  export let entry: LauncherEntry | null = null;
  export let x = 0;
  export let y = 0;
  export let inputActionItems: InputActionItem[] = [];
  export let availableEntryActions: (entry: LauncherEntry | null) => EntryActionItem[];
  export let onInputAction: (action: InputContextAction) => Promise<void> | void;
  export let onEntryAction: (action: ContextAction) => Promise<void> | void;
  export let onEscape: (event: KeyboardEvent) => void;
</script>

{#if open}
  <div
    class="contextMenu"
    role="menu"
    tabindex="-1"
    style={`left:${x}px; top:${y}px;`}
    on:click|stopPropagation
    on:keydown={onEscape}
  >
    {#if mode === 'input'}
      {#each inputActionItems as action}
        <button
          type="button"
          class="contextMenuItem"
          on:click={() => onInputAction(action.id)}
        >
          {action.label}
        </button>
      {/each}
    {:else if entry}
      {#each availableEntryActions(entry) as action}
        <button
          type="button"
          class="contextMenuItem"
          on:click={() => onEntryAction(action.id)}
        >
          {action.contextLabel}
        </button>
      {/each}
    {/if}
  </div>
{/if}
