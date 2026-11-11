<script lang="ts">
    import { getContext } from "svelte";
    import { page } from "$app/state";
    import { api } from "$lib/api/client";
    import type { components } from "$lib/api/schema";
    import LinkList from "$lib/components/LinkList.svelte";
    import { errorMessage, shortLabel } from "$lib/format";

    type Link = components["schemas"]["LinkResponse"];
    type UserInfo = components["schemas"]["UserInfo"];

    const user = getContext<() => UserInfo | null>("user");

    let links: Link[] = $state([]);
    let loaded = $state(false);
    let loadError: string | null = $state(null);
    let flash = $state(page.state.flash ?? null);

    let targetUrl = $state("");
    let slug = $state("");
    let creating = $state(false);

    async function load() {
        const { data, error } = await api.GET("/api/links");
        if (data) {
            links = data;
        } else {
            loadError = errorMessage(error, "Your links didn't load. Reload the page to try again.");
        }
        loaded = true;
    }

    async function create(event: SubmitEvent) {
        event.preventDefault();
        creating = true;

        const { data, error } = await api.POST("/api/links", {
            body: { target_url: targetUrl, slug: slug || undefined },
        });
        creating = false;

        if (!data) {
            flash = { kind: "error", text: errorMessage(error, "The link wasn't created. Try again.") };
            return;
        }

        links = [data, ...links];
        targetUrl = "";
        slug = "";
        flash = { kind: "success", text: `${shortLabel(data.slug)} is ready.` };
    }

    if (user()) {
        void load();
    }
</script>

{#if user()}
    <div class="content wide">
        <div>
            <h1 class="sr-only">{user()?.is_admin ? "All Links" : "Your Links"}</h1>
            {#if flash}
                <div class="cmu-flash cmu-flash--{flash.kind}" role={flash.kind === "error" ? "alert" : "status"}>{flash.text}</div>
            {/if}

            <!-- https://webkilt.andrew.cmu.edu/forms.html#field-row -->
            <form class="create-panel" onsubmit={create}>
                <h2>Create a Link</h2>
                <div class="cmu-field-row create">
                    <div>
                        <label for="new-target">Target URL</label>
                        <input type="text" inputmode="url" id="new-target" required bind:value={targetUrl} placeholder="https://" />
                    </div>
                    <div>
                        <label for="new-slug">Slug</label>
                        <input type="text" id="new-slug" bind:value={slug} placeholder="Random" />
                    </div>
                    <button class="btn" type="submit" disabled={creating}>Shorten</button>
                </div>
            </form>

            {#if loadError}
                <div class="cmu-flash cmu-flash--error" role="alert">{loadError}</div>
            {:else if loaded}
                <LinkList {links} isAdmin={user()?.is_admin ?? false} />
            {/if}
        </div>
    </div>
{:else}
    <div class="content">
        <div>
            <p class="cmu-lead">Sign in to create and manage {location.host} links.</p>
            <div class="cmu-cta-row">
                <a class="btn" href="/auth/login">Sign In</a>
            </div>
        </div>
    </div>
{/if}

<style>
    .create-panel {
        margin: 0 0 2em;
        padding: 1.5em;
        background: var(--cmu-tint);
    }

    :global(#content) .create-panel h2 {
        margin: 0 0 0.75em;
        padding: 0;
    }

    :global(#content) .create-panel .create {
        margin: 0;
    }

    .create > div:first-child {
        flex-grow: 4;
    }

    :global(#content) .create > .btn {
        box-sizing: border-box;
        height: 2.2em;
        margin: 0;
        padding-block: 0;
    }
</style>
