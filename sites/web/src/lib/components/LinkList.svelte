<script lang="ts">
    import { api } from "../api/client";
    import type { components } from "../api/schema";

    type Link = components["schemas"]["LinkResponse"];

    let {
        isAdmin = false,
        canModify = false,
    }: { isAdmin?: boolean; canModify?: boolean } = $props();

    let links: Link[] = $state([]);
    let loading = $state(true);
    let editingId: string | null = $state(null);
    let editSlug = $state("");
    let editTarget = $state("");
    let editError: string | null = $state(null);

    async function fetchLinks() {
        loading = true;
        try {
            const { data } = await api.GET("/api/links");
            if (data) {
                links = data;
            }
        } finally {
            loading = false;
        }
    }

    function startEdit(link: Link) {
        editingId = link.id;
        editSlug = link.slug;
        editTarget = link.target_url;
        editError = null;
    }

    function cancelEdit() {
        editingId = null;
        editError = null;
    }

    async function saveEdit(id: string) {
        editError = null;
        const { data, error } = await api.PATCH("/api/links/{id}", {
            params: { path: { id } },
            body: { slug: editSlug, target_url: editTarget },
        });
        if (!data) {
            editError = (error as { error?: string })?.error ?? "Failed to update link";
            return;
        }
        links = links.map((l) => (l.id === id ? data : l));
        editingId = null;
    }

    async function deleteLink(id: string) {
        const { error } = await api.DELETE("/api/links/{id}", {
            params: { path: { id } },
        });
        if (!error) {
            links = links.filter((l) => l.id !== id);
        }
    }

    function formatDate(iso: string): string {
        return new Date(iso).toLocaleString();
    }

    fetchLinks();
</script>

<section>
    <h2>{isAdmin ? "All links" : "Your links"}</h2>

    {#if loading}
        <p>Loading...</p>
    {:else if links.length === 0}
        <p>No links yet.</p>
    {:else}
        <ul>
            {#each links as link (link.id)}
                <li>
                    {#if editingId === link.id}
                        <input type="text" bind:value={editSlug} placeholder="slug" />
                        <input
                            type="url"
                            bind:value={editTarget}
                            placeholder="https://example.com"
                        />
                        <button onclick={() => saveEdit(link.id)}>Save</button>
                        <button onclick={cancelEdit}>Cancel</button>
                        {#if editError}
                            <span>{editError}</span>
                        {/if}
                    {:else}
                        <a href="/{link.slug}">/{link.slug}</a>
                        -> {link.target_url}
                        <span>created {formatDate(link.created_at)}</span>
                        {#if isAdmin && link.owner_name}
                            <span>by {link.owner_name}</span>
                        {/if}
                        {#if canModify}
                            <button onclick={() => startEdit(link)}>Edit</button>
                            <button onclick={() => deleteLink(link.id)}>Delete</button>
                        {/if}
                    {/if}
                </li>
            {/each}
        </ul>
    {/if}
</section>
