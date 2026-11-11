<script lang="ts" module>
    import { api } from "../api/client";

    const crawlerTags = api.GET("/api/crawler-tags").then(({ data }) => data ?? []);
</script>

<script lang="ts">
    import type { components } from "../api/schema";

    type Rule = components["schemas"]["RuleInput"];

    const PLATFORMS = [
        "ios",
        "android",
        "mobile",
        "desktop",
        "windows",
        "macos",
        "linux",
        "chromeos",
        "bot",
    ];

    let { rules = $bindable() }: { rules: Rule[] } = $props();

    function move(index: number, offset: number) {
        const target = index + offset;
        if (target < 0 || target >= rules.length) {
            return;
        }
        const reordered = [...rules];
        [reordered[index], reordered[target]] = [reordered[target], reordered[index]];
        rules = reordered;
    }

    async function setKind(index: number, kind: Rule["kind"]) {
        rules[index].kind = kind;
        if (kind === "platform") {
            rules[index].pattern = "ios";
        } else if (kind === "crawler") {
            rules[index].pattern = (await crawlerTags)[0] ?? "";
        } else {
            rules[index].pattern = "";
        }
    }
</script>

<fieldset>
    <legend>Redirect Rules</legend>
    <!-- https://webkilt.andrew.cmu.edu/forms.html#sort -->
    <div>
        <ol class="cmu-sort">
            {#each rules as rule, index (index)}
                <li>
                    <!-- https://webkilt.andrew.cmu.edu/forms.html#field-row -->
                    <div class="cmu-field-row rule">
                        <div>
                            <label for="rule-{index}-kind">Match</label>
                            <select
                                id="rule-{index}-kind"
                                value={rule.kind}
                                onchange={(e) => setKind(index, e.currentTarget.value as Rule["kind"])}
                            >
                                <option value="platform">Platform</option>
                                <option value="regex">Regex</option>
                                <option value="crawler">Crawler</option>
                            </select>
                        </div>
                        <div>
                            {#if rule.kind === "platform"}
                                <label for="rule-{index}-pattern">Platform</label>
                                <select id="rule-{index}-pattern" bind:value={rules[index].pattern}>
                                    {#each PLATFORMS as platform (platform)}
                                        <option value={platform}>{platform}</option>
                                    {/each}
                                </select>
                            {:else if rule.kind === "crawler"}
                                <label for="rule-{index}-pattern">Category</label>
                                {#await crawlerTags then tags}
                                    <select id="rule-{index}-pattern" bind:value={rules[index].pattern}>
                                        {#each tags as tag (tag)}
                                            <option value={tag}>{tag}</option>
                                        {/each}
                                    </select>
                                {/await}
                            {:else}
                                <label for="rule-{index}-pattern">Pattern</label>
                                <input
                                    id="rule-{index}-pattern"
                                    type="text"
                                    bind:value={rules[index].pattern}
                                    placeholder="CriOS/1[0-9]+"
                                />
                            {/if}
                        </div>
                        <div>
                            <label for="rule-{index}-target">Target URL</label>
                            <input
                                id="rule-{index}-target"
                                type="text"
                                bind:value={rules[index].target_url}
                                placeholder="https://apps.apple.com/app/id123"
                            />
                        </div>
                    </div>
                    <span class="cmu-sort-moves">
                        <button
                            class="btn ghost small"
                            type="button"
                            onclick={() => move(index, -1)}
                            disabled={index === 0}
                            aria-label="Move rule {index + 1} up"
                            title="Move up">{"\u25B2"}</button
                        >
                        <button
                            class="btn ghost small"
                            type="button"
                            onclick={() => move(index, 1)}
                            disabled={index === rules.length - 1}
                            aria-label="Move rule {index + 1} down"
                            title="Move down">{"\u25BC"}</button
                        >
                        <button
                            class="btn ghost small"
                            type="button"
                            onclick={() => (rules = rules.filter((_, i) => i !== index))}
                            aria-label="Remove rule {index + 1}"
                            title="Remove rule">{"\u2715"}</button
                        >
                    </span>
                </li>
            {/each}
        </ol>
    </div>

    <p class="rule-buttons">
        <button
            class="btn small ghost"
            type="button"
            onclick={() => (rules = [...rules, { kind: "platform", pattern: "ios", target_url: "" }])}
        >
            Add a&nbsp;Rule
        </button>
    </p>
</fieldset>

<style>
    :global(#content) ol.cmu-sort > li {
        align-items: flex-end;
    }

    :global(#content) ol.cmu-sort > li::before {
        align-self: center;
    }

    :global(#content) .rule {
        flex: 1;
        margin: 0;
    }

    :global(#content) .cmu-sort-moves .btn {
        display: grid;
        place-items: center;
        box-sizing: border-box;
        width: 2.2em;
        height: 2.2em;
        padding: 0;
        font-size: 1em;
    }

    :global(#content) p.rule-buttons {
        margin: 0;
        padding: 0;
    }

    @media (max-width: 767px) {
        :global(#content) ol.cmu-sort > li {
            flex-wrap: wrap;
        }

        :global(#content) .rule {
            flex-basis: 100%;
            order: 1;
        }
    }
</style>
