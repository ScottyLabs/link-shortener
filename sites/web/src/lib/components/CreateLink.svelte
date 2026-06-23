<script lang="ts">
    let { onCreated }: { onCreated: () => void } = $props();

    let targetUrl = $state("");
    let slug = $state("");
    let error: string | null = $state(null);
    let submitting = $state(false);

    async function submit() {
        error = null;
        submitting = true;
        try {
            const res = await fetch("/api/links", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    target_url: targetUrl,
                    slug: slug || undefined,
                }),
            });
            if (!res.ok) {
                const body = await res.json();
                error = body.error ?? "Failed to create link";
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
        <p class="error">{error}</p>
    {/if}
</form>
