<script lang="ts">
    type Link = {
        id: string;
        slug: string;
        target_url: string;
        created_at: string;
        updated_at: string;
    };

    let links: Link[] = $state([]);
    let loading = $state(true);

    async function fetchLinks() {
        loading = true;
        try {
            const res = await fetch("/api/links");
            if (res.ok) {
                links = await res.json();
            }
        } finally {
            loading = false;
        }
    }

    async function deleteLink(id: string) {
        await fetch(`/api/links/${id}`, { method: "DELETE" });
        links = links.filter((l) => l.id !== id);
    }

    fetchLinks();
</script>

<section>
    <h2>Your links</h2>

    {#if loading}
        <p>Loading...</p>
    {:else if links.length === 0}
        <p>No links yet.</p>
    {:else}
        <ul>
            {#each links as link (link.id)}
                <li>
                    <a href="/{link.slug}">/{link.slug}</a>
                    -> {link.target_url}
                    <button onclick={() => deleteLink(link.id)}>Delete</button>
                </li>
            {/each}
        </ul>
    {/if}
</section>
