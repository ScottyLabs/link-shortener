<script lang="ts">
    import { api } from "../api/client";

    let { onCreated }: { onCreated: () => void } = $props();

    let targetUrl = $state("");
    let slug = $state("");
    let error: string | null = $state(null);
    let submitting = $state(false);

    async function submit() {
        error = null;
        submitting = true;
        try {
            const { data, error: err } = await api.POST("/api/links", {
                body: { target_url: targetUrl, slug: slug || undefined },
            });
            if (!data) {
                error = (err as { error?: string })?.error ?? "Failed to create link";
                return;
            }
            targetUrl = "";
            slug = "";
            onCreated();
        } finally {
            submitting = false;
        }
    }
</script>

<form onsubmit={(e) => { e.preventDefault(); submit(); }}>
    <h2>Create a short link</h2>
    <label>
        Target URL
        <input type="url" bind:value={targetUrl} required placeholder="https://example.com" />
    </label>
    <label>
        Custom slug (optional)
        <input type="text" bind:value={slug} placeholder="my-link" />
    </label>
    <button type="submit" disabled={submitting}>
        {submitting ? "Creating..." : "Create"}
    </button>
    {#if error}
        <p>{error}</p>
    {/if}
</form>
