<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import {
    restoreStateCurrent,
    saveWindowState,
    StateFlags
  } from '@tauri-apps/plugin-window-state';

  onMount(() => {
    void restoreStateCurrent(StateFlags.ALL);

    const onBeforeUnload = () => {
      void saveWindowState(StateFlags.ALL);
    };

    window.addEventListener('beforeunload', onBeforeUnload);

    return () => {
      window.removeEventListener('beforeunload', onBeforeUnload);
    };
  });
</script>

<slot />
