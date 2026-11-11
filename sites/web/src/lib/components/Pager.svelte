<script lang="ts">
    import { page } from "$app/state";

    let { current, count }: { current: number; count: number } = $props();

    const slots = $derived.by((): (number | null)[] => {
        if (count <= 7) {
            return Array.from({ length: count }, (_, index) => index + 1);
        }
        if (current <= 4) {
            return [1, 2, 3, 4, 5, null, count];
        }
        if (current >= count - 3) {
            return [1, null, count - 4, count - 3, count - 2, count - 1, count];
        }
        return [1, null, current - 1, current, current + 1, null, count];
    });

    function href(target: number): string {
        const url = new URL(page.url);
        url.searchParams.set("page", String(target));
        return url.search;
    }
</script>

{#snippet arrow(target: number, enabled: boolean, label: string, symbol: string, rel: string | undefined)}
    {#if enabled}
        <a class="cmu-pager-arrow" href={href(target)} {rel} aria-label={label}>{symbol}</a>
    {:else}
        <!-- svelte-ignore a11y_missing_attribute -->
        <a class="cmu-pager-arrow" aria-disabled="true" aria-label={label}>{symbol}</a>
    {/if}
{/snippet}

<!-- https://webkilt.andrew.cmu.edu/lists.html#pagination -->
<div class="cmu-pager" role="navigation" aria-label="Pages">
    <ol>
        <li class="cmu-pager-end">{@render arrow(1, current > 1, "First page", "\u00ab", undefined)}</li>
        <li>{@render arrow(current - 1, current > 1, "Previous page", "\u2039", "prev")}</li>
        {#each slots as slot, index (index)}
            <li>
                {#if slot === null}
                    <span>&hellip;</span>
                {:else if slot === current}
                    <a href={href(slot)} aria-current="page">{slot}</a>
                {:else}
                    <a href={href(slot)}>{slot}</a>
                {/if}
            </li>
        {/each}
        <li>{@render arrow(current + 1, current < count, "Next page", "\u203a", "next")}</li>
        <li class="cmu-pager-end">{@render arrow(count, current < count, "Last page", "\u00bb", undefined)}</li>
    </ol>
</div>
