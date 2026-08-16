<script lang="ts">
  import { onMount } from 'svelte';
  import Cockpit from '$lib/components/shell/Cockpit.svelte';
  import { loadConfig } from '$lib/stores/config';

  onMount(loadConfig);
</script>

<Cockpit />
