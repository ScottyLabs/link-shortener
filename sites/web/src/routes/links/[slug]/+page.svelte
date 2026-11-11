<script lang="ts">
    import { getContext } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { api } from "$lib/api/client";
    import type { components } from "$lib/api/schema";
    import DeleteDialog from "$lib/components/DeleteDialog.svelte";
    import RuleEditor from "$lib/components/RuleEditor.svelte";
    import { errorMessage, shortLabel } from "$lib/format";

    type Link = components["schemas"]["LinkResponse"];
    type Rule = components["schemas"]["RuleInput"];

    const shared = getContext<{ link: Link | null; flash: App.PageState["flash"] | null }>("link");

    let targetUrl = $state("");
    let slug = $state("");
    let rules: Rule[] = $state([]);
    let paused = $state(false);
    let expires = $state("");
    let saving = $state(false);

    let deleteDialog: ReturnType<typeof DeleteDialog> | undefined = $state();

    function toLocalInput(iso: string): string {
        const date = new Date(iso);
        return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16);
    }

    $effect(() => {
        if (shared.link) {
            targetUrl = shared.link.target_url;
            slug = shared.link.slug;
            rules = shared.link.rules.map(({ kind, pattern, target_url }) => ({ kind, pattern, target_url }));
            paused = shared.link.paused;
            expires = shared.link.expires_at ? toLocalInput(shared.link.expires_at) : "";
        }
    });

    async function save(event: SubmitEvent) {
        event.preventDefault();
        if (!shared.link) {
            return;
        }
        saving = true;

        const { data, error } = await api.PATCH("/api/links/{id}", {
            params: { path: { id: shared.link.id } },
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
            shared.flash = { kind: "error", text: errorMessage(error, "Your changes weren't saved. Try again.") };
            return;
        }

        shared.link = data;
        shared.flash = { kind: "success", text: "Your changes are saved." };
        if (data.slug !== page.params.slug) {
            await goto(`/links/${data.slug}`, { replaceState: true, keepFocus: true, noScroll: true });
        }
    }

    function deleted(gone: Link) {
        void goto("/", { state: { flash: { kind: "success", text: `${shortLabel(gone.slug)} was deleted.` } } });
    }
</script>

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
        <button class="btn ghost" type="button" onclick={() => shared.link && deleteDialog?.open(shared.link)}>Delete Link</button>
    </p>
</form>

<DeleteDialog bind:this={deleteDialog} onDeleted={deleted} />

<style>
    .wide-first > div:first-child {
        flex-grow: 3;
    }

    :global(#content) .settings > div {
        flex: 0 1 16em;
    }
</style>
