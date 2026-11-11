<script lang="ts">
    import { api } from "../api/client";
    import type { components } from "../api/schema";
    import { errorMessage, shortLabel } from "../format";

    type Link = components["schemas"]["LinkResponse"];

    let { onDeleted }: { onDeleted: (link: Link) => void } = $props();

    let dialog: HTMLDialogElement;
    let link: Link | null = $state(null);
    let error: string | null = $state(null);
    let deleting = $state(false);

    export function open(target: Link) {
        link = target;
        error = null;
        dialog.showModal();
    }

    async function submit(event: SubmitEvent) {
        if (event.submitter?.getAttribute("value") !== "delete" || !link) {
            return;
        }
        event.preventDefault();
        error = null;
        deleting = true;

        const { error: err } = await api.DELETE("/api/links/{id}", { params: { path: { id: link.id } } });
        deleting = false;

        if (err) {
            error = errorMessage(err, "The link wasn't deleted. Try again.");
            return;
        }
        onDeleted(link);
        dialog.close();
    }
</script>

<!-- https://webkilt.andrew.cmu.edu/messages.html#dialog -->
<dialog class="cmu-dialog" bind:this={dialog} aria-labelledby="delete-dialog-title">
    <form method="dialog" onsubmit={submit}>
        <h2 id="delete-dialog-title">Delete {link ? shortLabel(link.slug) : "the Link"}?</h2>
        {#if error}
            <div class="cmu-flash cmu-flash--error" role="alert">{error}</div>
        {/if}
        <p>This link will stop working for everyone.</p>
        <div class="cmu-dialog-buttons">
            <button class="btn" type="submit" value="delete" disabled={deleting}>Delete Link</button>
            <button class="btn ghost" type="submit" value="cancel">Cancel</button>
        </div>
    </form>
</dialog>
