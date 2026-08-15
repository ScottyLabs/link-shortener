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

    function setKind(index: number, kind: Rule["kind"]) {
        rules[index].kind = kind;
        rules[index].pattern = kind === "platform" ? "ios" : "";
    }
</script>

<fieldset>
    <legend>User-agent rules</legend>
    <p>Applied in order from top to bottom.</p>

    {#each rules as rule, index (index)}
        <div>
            <select
                value={rule.kind}
                onchange={(e) => setKind(index, e.currentTarget.value as Rule["kind"])}
            >
                <option value="platform">Platform</option>
                <option value="regex">Regex</option>
            </select>

            {#if rule.kind === "platform"}
                <select bind:value={rules[index].pattern}>
                    {#each PLATFORMS as platform (platform)}
                        <option value={platform}>{platform}</option>
                    {/each}
                </select>
            {:else}
                <input
                    type="text"
                    bind:value={rules[index].pattern}
                    placeholder="CriOS/1[0-9]+"
                />
            {/if}

            <input
                type="text"
                bind:value={rules[index].target_url}
                placeholder="https://apps.apple.com/app/id123"
            />

            <button type="button" onclick={() => move(index, -1)} disabled={index === 0}>
                Up
            </button>
            <button
                type="button"
                onclick={() => move(index, 1)}
                disabled={index === rules.length - 1}
            >
                Down
            </button>
            <button type="button" onclick={() => (rules = rules.filter((_, i) => i !== index))}>
                Remove
            </button>
        </div>
    {/each}

    <button
        type="button"
        onclick={() => (rules = [...rules, { kind: "platform", pattern: "ios", target_url: "" }])}
    >
        Add rule
    </button>
</fieldset>
