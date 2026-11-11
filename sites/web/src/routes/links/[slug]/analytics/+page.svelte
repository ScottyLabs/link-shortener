<script lang="ts">
    import { getContext } from "svelte";
    import { BarChart, Tooltip, defaultChartPadding } from "layerchart";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { api } from "$lib/api/client";
    import type { components, operations } from "$lib/api/schema";
    import { apDate, apMonthDay } from "$lib/format";

    type Link = components["schemas"]["LinkResponse"];
    type LinkStats = components["schemas"]["LinkStats"];
    type StatsQuery = NonNullable<operations["link_stats"]["parameters"]["query"]>;
    type Range = NonNullable<StatsQuery["range"]>;
    type Dimension = keyof LinkStats["breakdowns"];

    const shared = getContext<{ link: Link | null }>("link");

    let stats = $state<LinkStats | null>(null);
    let chartWidth = $state(0);
    let latest = 0;

    const RANGES: { value: Range; label: string; phrase: string }[] = [
        { value: "7d", label: "Last 7 Days", phrase: "in the last 7 days" },
        { value: "30d", label: "Last 30 Days", phrase: "in the last 30 days" },
        { value: "90d", label: "Last 90 Days", phrase: "in the last 90 days" },
        { value: "all", label: "All Time", phrase: "since the link was created" },
    ];

    const FILTERS: { key: Dimension; label: string }[] = [
        { key: "traffic", label: "Traffic" },
        { key: "source", label: "Source" },
        { key: "device_type", label: "Device" },
        { key: "os", label: "OS" },
        { key: "browser", label: "Browser" },
        { key: "country", label: "Country" },
        { key: "city", label: "City" },
    ];

    const TRAFFIC: Record<string, string> = { all: "All Traffic", people: "People", bots: "Bots" };

    // PostHog's browser IDs and our in_app_browser values
    const BROWSERS: Record<string, string> = {
        ios: "Safari on iOS",
        safari: "Safari",
        crios: "Chrome on iOS",
        chrome: "Chrome",
        fxios: "Firefox on iOS",
        firefox: "Firefox",
        "edge-ios": "Edge on iOS",
        "edge-chromium": "Edge",
        samsung: "Samsung Internet",
        opera: "Opera",
        instagram: "Instagram",
        facebook: "Facebook",
        messenger: "Messenger",
        threads: "Threads",
        twitter: "X",
        tiktok: "TikTok",
        snapchat: "Snapchat",
        linkedin: "LinkedIn",
        reddit: "Reddit",
        whatsapp: "WhatsApp",
        wechat: "WeChat",
        line: "LINE",
        google: "Google",
    };

    const range = $derived(RANGES.find((option) => option.value === page.url.searchParams.get("range")) ?? RANGES[1]);

    const query = $derived.by(() => {
        const query: StatsQuery = { range: range.value };
        for (const { key } of FILTERS) {
            const values = page.url.searchParams.getAll(key);
            if (values.length > 0) {
                query[key] = values;
            }
        }
        return query;
    });

    $effect(() => {
        if (shared.link) {
            void loadStats(shared.link.id, query);
        }
    });

    function selected(key: Dimension): string[] {
        return query[key] ?? (key === "traffic" ? ["people"] : []);
    }

    function filter(key: Dimension | "range", values: string[]) {
        const params = new URLSearchParams(page.url.searchParams);
        params.delete(key);
        if (key === "traffic" && values.length === 0) {
            params.set(key, "all");
        } else if (!(key === "traffic" && values.length === 1 && values[0] === "people")) {
            for (const value of values) {
                params.append(key, value);
            }
        }
        void goto(`?${params}`, { replaceState: true, noScroll: true, keepFocus: true });
    }

    function toggle(key: Dimension, value: string) {
        const current = selected(key).filter((other) => other !== "all");
        filter(key, current.includes(value) ? current.filter((other) => other !== value) : [...current, value]);
    }

    function options(key: Dimension): string[] {
        const values = stats?.breakdowns[key].map(({ value }) => value ?? "unknown") ?? [];
        const always = key === "traffic" ? Object.keys(TRAFFIC) : [];
        return [...new Set([...always, ...selected(key), ...values])];
    }

    function valueLabel(key: Dimension, value?: string | null): string {
        if (value == null || (key !== "traffic" && value === "unknown")) {
            return key === "source" ? "None" : "Unknown";
        }
        if (key === "source" && value === "qr") {
            return "QR Code";
        }
        if (key === "traffic") {
            return TRAFFIC[value] ?? value;
        }
        return key === "browser" ? (BROWSERS[value] ?? value) : value;
    }

    async function loadStats(id: string, wanted: StatsQuery) {
        const request = ++latest;
        const { data } = await api.GET("/api/links/{id}/stats", { params: { path: { id }, query: wanted } });
        if (request === latest) {
            stats = data ?? null;
        }
    }

    function parseDay(start: string): Date {
        const [year, month, day] = start.split("-").map(Number);
        return new Date(year, month - 1, day);
    }

    function barLabel(start: string, bucket: LinkStats["bucket"]): string {
        const date = parseDay(start);
        if (bucket === "month") {
            return date.toLocaleString("en-US", { month: "long", year: "numeric" });
        }
        return bucket === "week" ? `Week of ${apDate(date)}` : apDate(date);
    }

    function axisLabel(start: string, bucket: LinkStats["bucket"]): string {
        const date = parseDay(start);
        return bucket === "month" ? date.toLocaleString("en-US", { month: "long" }) : apMonthDay(date);
    }
</script>

{#if stats}
    {@const bucket = stats.bucket}
    <!-- https://webkilt.andrew.cmu.edu/lists.html#filters -->
    <div class="cmu-filters">
        <div class="cmu-filter-group">
            <label class="cmu-label" for="filter-range">Period</label>
            <select id="filter-range" value={range.value} onchange={(event) => filter("range", [event.currentTarget.value])}>
                {#each RANGES as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                {/each}
            </select>
        </div>
        {#each FILTERS as { key, label } (key)}
            {@const values = selected(key)}
            <div class="cmu-filter-group">
                <label class="cmu-label" for="filter-{key}">{label}</label>
                <select
                    id="filter-{key}"
                    value={values.length > 1 ? "several" : (values[0] ?? "")}
                    onchange={(event) => filter(key, event.currentTarget.value ? [event.currentTarget.value] : [])}
                >
                    {#if values.length > 1}
                        <option value="several" disabled>{values.map((value) => valueLabel(key, value)).join(", ")}</option>
                    {/if}
                    {#if key !== "traffic"}
                        <option value="">Any</option>
                    {/if}
                    {#each options(key) as value (value)}
                        <option {value}>{valueLabel(key, value)}</option>
                    {/each}
                </select>
            </div>
        {/each}
    </div>
    <p>
        <strong>{stats.visits.toLocaleString("en-US")}</strong>
        {stats.visits === 1 ? "visit" : "visits"} from
        <strong>{stats.visitors.toLocaleString("en-US")}</strong>
        {stats.visitors === 1 ? "visitor" : "visitors"}
        {range.phrase}
    </p>
    {#if stats.visits > 0}
        <div class="chart" bind:clientWidth={chartWidth}>
            <BarChart
                data={stats.bars}
                x="start"
                y="visits"
                height={200}
                padding={defaultChartPadding({ left: 36 })}
                props={{
                    xAxis: {
                        format: (start: string) => axisLabel(start, bucket),
                        ticks: (scale: { domain: () => string[] }) =>
                            scale
                                .domain()
                                .filter(
                                    (_, index, all) =>
                                        index % Math.ceil(all.length / Math.max(2, Math.floor(chartWidth / 100))) === 0,
                                ),
                    },
                }}
            >
                {#snippet tooltip()}
                    <Tooltip.Root portal={false}>
                        {#snippet children({ data })}
                            <Tooltip.Header>{barLabel(data.start, bucket)}</Tooltip.Header>
                            <Tooltip.List>
                                <Tooltip.Item label="Visits" value={data.visits} />
                            </Tooltip.List>
                        {/snippet}
                    </Tooltip.Root>
                {/snippet}
            </BarChart>
        </div>
    {/if}
    <div class="panel">
        <!-- https://webkilt.andrew.cmu.edu/layout.html#tabs -->
        <div class="cmu-tabs" role="tablist" aria-label="Visits by">
            {#each FILTERS as { key, label } (key)}
                <button
                    type="button"
                    role="tab"
                    id="visits-{key}-tab"
                    aria-controls="visits-{key}"
                    aria-selected={key === FILTERS[0].key}>{label}</button
                >
            {/each}
        </div>
        {#each FILTERS as { key, label } (key)}
            {@const most = Math.max(1, ...stats.breakdowns[key].map((row) => row.visits))}
            {@const values = selected(key)}
            <div role="tabpanel" id="visits-{key}" aria-labelledby="visits-{key}-tab" hidden={key !== FILTERS[0].key}>
                <!-- https://webkilt.andrew.cmu.edu/messages.html#progress -->
                <ul class="cmu-breakdown" aria-label={label}>
                    {#each stats.breakdowns[key] as row (row.value)}
                        {@const value = row.value ?? "unknown"}
                        <li>
                            <label>
                                <span>
                                    <input
                                        type="checkbox"
                                        {value}
                                        checked={values.includes(value)}
                                        onchange={() => toggle(key, value)}
                                    />
                                    {valueLabel(key, value)}
                                </span>
                                <div class="cmu-meter cmu-meter--full"><div style:width="{(row.visits / most) * 100}%"></div></div>
                                <span class="cmu-pct">{row.visits.toLocaleString("en-US")}</span>
                            </label>
                        </li>
                    {/each}
                </ul>
            </div>
        {/each}
    </div>
{/if}

<style>
    .cmu-filters {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
    }

    @media (max-width: 767px) {
        .cmu-filters {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }
    }

    :global(#content) .cmu-filters select {
        width: 100%;
        margin: 0;
    }

    .panel {
        margin: 1.5em 0 0;
    }

    .panel .cmu-tabs {
        flex-wrap: nowrap;
        overflow: auto hidden;
        scrollbar-width: none;
    }

    .panel .cmu-tabs button {
        flex: none;
    }

    .cmu-breakdown {
        display: grid;
        grid-template-columns: min(12em, 40%) 1fr 4em;
        column-gap: 0.75em;
        border-top: 0;
    }

    .cmu-breakdown li,
    .cmu-breakdown label {
        display: grid;
        grid-column: 1 / -1;
        grid-template-columns: subgrid;
        align-items: center;
        margin: 0;
    }

    .cmu-breakdown label {
        padding: 0.35em 0.5em;
        cursor: pointer;
    }

    .cmu-breakdown label:hover {
        background: var(--cmu-tint);
    }

    .cmu-breakdown label:has(:checked) {
        background: var(--cmu-red-tint);
    }

    .cmu-breakdown input {
        margin: 0 0.5em 0 0;
    }

    .cmu-meter > div {
        background: var(--cmu-red);
    }

    .cmu-breakdown .cmu-pct {
        text-align: right;
    }

    .chart :global(.lc-root-container) {
        --color-primary: var(--cmu-red);
        --color-surface-100: #fff;
        --color-surface-200: var(--cmu-tint);
        --color-surface-300: var(--cmu-line);
        --color-surface-content: var(--cmu-ink);
    }

    .chart :global(.lc-bar) {
        stroke: none;
    }

    .chart :global(.lc-text-svg) {
        overflow: visible;
    }

    .chart :global(.lc-axis-tick-label) {
        font-size: 12px;
        font-weight: 400;
    }
</style>
