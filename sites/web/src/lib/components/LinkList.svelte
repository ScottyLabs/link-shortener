<script lang="ts">
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import type { components } from "../api/schema";
    import { apDate, shortLabel, shortUrl } from "../format";
    import Pager from "./Pager.svelte";

    type Link = components["schemas"]["LinkResponse"];
    type Status = "active" | "paused" | "expired";

    const PAGE_SIZE = 20;

    const STATUSES: { value: Status; label: string }[] = [
        { value: "active", label: "Active" },
        { value: "paused", label: "Paused" },
        { value: "expired", label: "Expired" },
    ];

    let { links, isAdmin }: { links: Link[]; isAdmin: boolean } = $props();

    const query = $derived(page.url.searchParams.get("q") ?? "");
    const statusFilter = $derived(page.url.searchParams.get("status") ?? "");

    function statusOf(link: Link): Status {
        if (link.paused) {
            return "paused";
        }
        if (link.expires_at && new Date(link.expires_at) <= new Date()) {
            return "expired";
        }
        return "active";
    }

    const filtered = $derived.by(() => {
        const needle = query.trim().toLowerCase();
        return links
            .filter((link) => statusFilter === "" || statusOf(link) === statusFilter)
            .filter(
                (link) =>
                    needle === "" ||
                    [link.slug, link.target_url, link.owner_name ?? ""].some((field) =>
                        field.toLowerCase().includes(needle),
                    ),
            )
            .sort((a, b) => b.created_at.localeCompare(a.created_at));
    });

    const pageCount = $derived(Math.max(1, Math.ceil(filtered.length / PAGE_SIZE)));
    const current = $derived(
        Math.min(pageCount, Math.max(1, Number(page.url.searchParams.get("page")) || 1)),
    );
    const shown = $derived(filtered.slice((current - 1) * PAGE_SIZE, current * PAGE_SIZE));

    const count = $derived.by(() => {
        if (filtered.length === 1) {
            return "Showing 1 link";
        }
        if (filtered.length <= PAGE_SIZE) {
            return `Showing all ${filtered.length} links`;
        }
        const first = (current - 1) * PAGE_SIZE + 1;
        const last = Math.min(current * PAGE_SIZE, filtered.length);
        return `Showing ${first}-${last} of ${filtered.length} links`;
    });

    function navigate(change: (params: URLSearchParams) => void) {
        const url = new URL(page.url);
        change(url.searchParams);
        url.searchParams.delete("page");
        void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
    }

    function setQuery(value: string) {
        navigate((params) => (value ? params.set("q", value) : params.delete("q")));
    }

    function setStatus(value: string) {
        navigate((params) => (value ? params.set("status", value) : params.delete("status")));
    }
</script>

{#if links.length === 0}
    <p class="cmu-empty">{isAdmin ? "No one has created a link yet." : "You haven't created any links yet."}</p>
{:else}
    <!-- https://webkilt.andrew.cmu.edu/lists.html#filters -->
    <div class="cmu-filters">
        <div class="cmu-filter-group">
            <label class="cmu-label" for="find-links">Find Links</label>
            <input
                type="text"
                id="find-links"
                placeholder="Slug, target or owner"
                value={query}
                oninput={(event) => setQuery(event.currentTarget.value)}
            />
        </div>
        <div class="cmu-filter-group">
            <label class="cmu-label" for="link-status">Status</label>
            <select id="link-status" value={statusFilter} onchange={(event) => setStatus(event.currentTarget.value)}>
                <option value="">All</option>
                {#each STATUSES as status (status.value)}
                    <option value={status.value}>{status.label}</option>
                {/each}
            </select>
        </div>
    </div>

    {#if filtered.length === 0}
        <p class="cmu-empty">No links match these filters.</p>
    {:else}
        <ul class="links">
            {#each shown as link (link.id)}
                {@const status = statusOf(link)}
                <li>
                    <div>
                        <p>
                            <a class="slug" href="/links/{link.slug}">
                                <data id="link-{link.id}" value={shortUrl(link.slug)}>{link.slug}</data>
                            </a>
                        </p>
                        <p class="cmu-muted">
                            {link.target_url}{#if isAdmin && link.owner_name}&ensp;&middot;&ensp;{link.owner_name}{/if}
                        </p>
                    </div>
                    <div class="meta">
                        {#if status === "paused"}
                            <span class="cmu-pill">Paused</span>
                        {:else if status === "expired"}
                            <span class="cmu-pill cmu-pill--amber">Expired</span>
                        {/if}
                        <span class="cmu-muted">{apDate(new Date(`${link.created_at}Z`))}</span>
                        <!-- https://webkilt.andrew.cmu.edu/content.html#copy -->
                        <button class="btn small" type="button" data-cmu-copy="#link-{link.id}">Copy Link</button>
                    </div>
                </li>
            {/each}
        </ul>
        <p class="cmu-results-count" aria-live="polite">{count}</p>
        {#if pageCount > 1}
            <Pager {current} count={pageCount} />
        {/if}
    {/if}
{/if}

<style>
    :global(#content) #link-status {
        margin: 0;
    }

    .links {
        margin: 0 0 1em;
        padding: 0;
        list-style: none;
    }

    .links > li {
        display: flex;
        align-items: center;
        gap: 1em;
        margin: 0;
        padding: 0.75em 0;
        border-top: 1px solid var(--cmu-line);
    }

    .links > li:last-child {
        border-bottom: 1px solid var(--cmu-line);
    }

    .links > li::before {
        content: none;
    }

    .links > li > div:first-child {
        flex: 1;
        min-width: 0;
    }

    .links p {
        margin: 0;
        padding: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .slug {
        font-weight: 600;
    }

    .meta {
        display: flex;
        align-items: center;
        gap: 0.75em;
        white-space: nowrap;
    }
</style>
