<script lang="ts">
    import { getContext } from "svelte";
    import { renderSVG } from "uqr";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { api } from "$lib/api/client";
    import type { components } from "$lib/api/schema";
    import DeleteDialog from "$lib/components/DeleteDialog.svelte";
    import RuleEditor from "$lib/components/RuleEditor.svelte";
    import { errorMessage, shortLabel, shortUrl } from "$lib/format";

    type Link = components["schemas"]["LinkResponse"];
    type Rule = components["schemas"]["RuleInput"];
    type UserInfo = components["schemas"]["UserInfo"];

    const user = getContext<() => UserInfo | null>("user");

    let link = $state<Link | null>(null);
    let loaded = $state(false);
    let loadError: string | null = $state(null);
    let flash = $state(page.state.flash ?? null);

    let targetUrl = $state("");
    let slug = $state("");
    let rules: Rule[] = $state([]);
    let paused = $state(false);
    let expires = $state("");
    let saving = $state(false);

    let deleteDialog: ReturnType<typeof DeleteDialog> | undefined = $state();

    const qrCode = $derived(
        link ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(renderSVG(shortUrl(link.slug)))}` : "",
    );

    function toLocalInput(iso: string): string {
        const date = new Date(iso);
        return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16);
    }

    function fill(next: Link) {
        link = next;
        targetUrl = next.target_url;
        slug = next.slug;
        rules = next.rules.map(({ kind, pattern, target_url }) => ({ kind, pattern, target_url }));
        paused = next.paused;
        expires = next.expires_at ? toLocalInput(next.expires_at) : "";
    }

    async function load() {
        const { data, error } = await api.GET("/api/links");
        if (!data) {
            loadError = errorMessage(error, "The link didn't load. Reload the page to try again.");
            loaded = true;
            return;
        }

        const wanted = page.params.slug?.toLowerCase();
        const found = data.find((candidate) => candidate.slug.toLowerCase() === wanted);
        if (found) {
            fill(found);
        }
        loaded = true;
    }

    async function save(event: SubmitEvent) {
        event.preventDefault();
        if (!link) {
            return;
        }
        saving = true;

        const { data, error } = await api.PATCH("/api/links/{id}", {
            params: { path: { id: link.id } },
            body: {
                target_url: targetUrl,
                slug,
                rules,
                paused,
                expires_at: expires ? new Date(expires).toISOString() : null,
            },
        });
        saving = false;

        if (!data) {
            flash = { kind: "error", text: errorMessage(error, "Your changes weren't saved. Try again.") };
            return;
        }

        fill(data);
        flash = { kind: "success", text: "Your changes are saved." };
        if (data.slug !== page.params.slug) {
            await goto(`/links/${data.slug}`, { replaceState: true, keepFocus: true, noScroll: true });
        }
    }

    function deleted(gone: Link) {
        void goto("/", { state: { flash: { kind: "success", text: `${shortLabel(gone.slug)} was deleted.` } } });
    }

    if (user()) {
        void load();
    }
</script>

<!-- https://webkilt.andrew.cmu.edu/layout.html#breadcrumbs -->
<div class="breadcrumbs">
    <a href="/">Link Shortener</a>&nbsp;&rsaquo;&nbsp;{link?.slug ?? page.params.slug}
</div>

{#if !user()}
    <div class="content">
        <div>
            <p class="cmu-lead">Sign in to manage this link.</p>
            <div class="cmu-cta-row">
                <a class="btn" href="/auth/login">Sign In</a>
            </div>
        </div>
    </div>
{:else if loadError}
    <div class="content">
        <div>
            <div class="cmu-flash cmu-flash--error" role="alert">{loadError}</div>
        </div>
    </div>
{:else if loaded && !link}
    <!-- https://webkilt.andrew.cmu.edu/messages.html#errors -->
    <div class="content">
        <div>
            <h1>Not Found</h1>
            <p class="cmu-lead">You don't have a link called {page.params.slug}.</p>
            <div class="cmu-cta-row">
                <a class="btn" href="/">See Your Links</a>
            </div>
        </div>
    </div>
{:else if link}
    <div class="content wide">
        <div>
            {#if flash}
                <div class="cmu-flash cmu-flash--{flash.kind}" role={flash.kind === "error" ? "alert" : "status"}>{flash.text}</div>
            {/if}

            <div class="summary">
                <img src={qrCode} alt="QR code for {shortLabel(link.slug)}" width="120" height="120" />
                <div>
                    <div class="title">
                        <h1>{link.slug}</h1>
                        {#if link.paused}
                            <span class="cmu-pill">Paused</span>
                        {:else if link.expires_at && new Date(link.expires_at) <= new Date()}
                            <span class="cmu-pill cmu-pill--amber">Expired</span>
                        {:else}
                            <span class="cmu-pill cmu-pill--green">Active</span>
                        {/if}
                    </div>
                    <p class="cmu-muted">Redirects to <a class="external" href={link.target_url}>{link.target_url}</a></p>
                    <data id="link-url" value={shortUrl(link.slug)} hidden></data>
                    <p class="buttons">
                        <!-- https://webkilt.andrew.cmu.edu/content.html#copy -->
                        <button type="button" class="btn small" data-cmu-copy="#link-url">Copy Link</button>
                        <a class="btn ghost small" href={qrCode} download="{link.slug}.svg">Download QR Code</a>
                    </p>
                </div>
            </div>

            <!-- https://webkilt.andrew.cmu.edu/forms.html#field-row -->
            <form onsubmit={save}>
                <div class="cmu-field-row wide-first">
                    <div>
                        <label for="link-target">Target URL</label>
                        <input type="text" inputmode="url" id="link-target" required bind:value={targetUrl} />
                    </div>
                    <div>
                        <label for="link-slug">Slug</label>
                        <input type="text" id="link-slug" required bind:value={slug} />
                    </div>
                </div>

                <div class="cmu-field-row settings">
                    <div>
                        <label for="link-status">Status</label>
                        <select
                            id="link-status"
                            value={paused ? "paused" : "active"}
                            onchange={(event) => (paused = event.currentTarget.value === "paused")}
                        >
                            <option value="active">Active</option>
                            <option value="paused">Paused</option>
                        </select>
                    </div>
                    <div>
                        <label for="link-expires">Expires</label>
                        <input type="datetime-local" id="link-expires" bind:value={expires} aria-describedby="link-expires-hint" />
                    </div>
                </div>
                <p class="cmu-hint" id="link-expires-hint">Leave Expires blank if the link shouldn't expire.</p>

                <RuleEditor bind:rules />

                <p class="buttons">
                    <button class="btn" type="submit" disabled={saving}>Save Changes</button>
                    <button class="btn ghost" type="button" onclick={() => link && deleteDialog?.open(link)}>Delete Link</button>
                </p>
            </form>
        </div>
    </div>

    <DeleteDialog bind:this={deleteDialog} onDeleted={deleted} />
{/if}

<style>
    .wide-first > div:first-child {
        flex-grow: 3;
    }

    :global(#content) .settings > div {
        flex: 0 1 16em;
    }

    .summary {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 1.5em;
        margin: 0 0 2em;
        padding: 1.25em;
        border: 1px solid var(--cmu-line);
    }

    .summary img {
        flex: none;
    }

    .summary > div {
        flex: 1 1 16em;
        min-width: 0;
    }

    :global(#content) .summary .btn {
        white-space: nowrap;
    }

    .title {
        display: flex;
        align-items: center;
        gap: 0.75em;
    }

    :global(#content) .summary h1 {
        margin: 0;
        padding: 0;
        font-size: 1.75em;
        overflow-wrap: anywhere;
    }

    :global(#content) .summary p {
        margin: 0.5em 0 0;
        padding: 0;
        overflow-wrap: anywhere;
    }

    :global(#content) p.buttons {
        display: flex;
        gap: 0.5em;
        padding: 0;
    }

    :global(#content) .buttons .btn {
        margin: 0;
    }
</style>
