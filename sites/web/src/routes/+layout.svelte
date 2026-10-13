<script lang="ts">
    import { initAnalytics } from "$lib/analytics";

    let { children } = $props();

    initAnalytics();
</script>

{@render children()}
