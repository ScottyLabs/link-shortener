<script lang="ts">
    import { getContext, setContext, untrack } from "svelte";
    import { renderSVG } from "uqr";
    import { page } from "$app/state";
    import { api } from "$lib/api/client";
    import type { components } from "$lib/api/schema";
    import { errorMessage, shortLabel, shortUrl } from "$lib/format";

    type Link = components["schemas"]["LinkResponse"];
    type UserInfo = components["schemas"]["UserInfo"];

    let { children } = $props();

    const user = getContext<() => UserInfo | null>("user");

    const current = $state<{ link: Link | null; flash: App.PageState["flash"] | null }>({
        link: null,
        flash: page.state.flash ?? null,
    });
    let loaded = $state(false);
    let loadError: string | null = $state(null);

    setContext("link", current);

    const analytics = $derived(page.route.id === "/links/[slug]/analytics");

    const qrCode = $derived(
        current.link
            ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(renderSVG(`${shortUrl(current.link.slug)}?utm_source=qr`))}`
            : "",
    );

    async function load(wanted: string) {
        const { data, error } = await api.GET("/api/links");
        if (!data) {
            loadError = errorMessage(error, "The link didn't load. Reload the page to try again.");
            loaded = true;
            return;
        }

        current.link = data.find((candidate) => candidate.slug.toLowerCase() === wanted) ?? null;
        loaded = true;
    }

    $effect(() => {
        const wanted = page.params.slug?.toLowerCase();
        if (user() && wanted && untrack(() => current.link?.slug.toLowerCase()) !== wanted) {
            void load(wanted);
        }
    });
</script>

<!-- https://webkilt.andrew.cmu.edu/layout.html#breadcrumbs -->
<div class="breadcrumbs">
    <a href="/">Link Shortener</a>&nbsp;&rsaquo;&nbsp;{#if analytics}<a href="/links/{page.params.slug}">{current.link?.slug ?? page.params.slug}</a
        >&nbsp;&rsaquo;&nbsp;Analytics{:else}{current.link?.slug ?? page.params.slug}{/if}
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
{:else if loaded && !current.link}
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
{:else if current.link}
    {@const link = current.link}
    <div class="content wide">
        <div>
            {#if current.flash}
                <div class="cmu-flash cmu-flash--{current.flash.kind}" role={current.flash.kind === "error" ? "alert" : "status"}>
                    {current.flash.text}
                </div>
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

            <!-- https://webkilt.andrew.cmu.edu/layout.html#tabs -->
            <div class="cmu-tabs" role="navigation" aria-label="Link">
                <a href="/links/{link.slug}" aria-current={analytics ? undefined : "page"} onclick={() => (current.flash = null)}
                    >Configuration</a
                >
                <a href="/links/{link.slug}/analytics" aria-current={analytics ? "page" : undefined} onclick={() => (current.flash = null)}
                    >Analytics</a
                >
            </div>

            {@render children()}
        </div>
    </div>
{/if}

<style>
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
