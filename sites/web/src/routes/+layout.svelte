<script lang="ts">
    import { setContext, tick } from "svelte";
    import { identify, initAnalytics, resetAnalytics } from "$lib/analytics";
    import { api } from "$lib/api/client";
    import type { components } from "$lib/api/schema";

    type UserInfo = components["schemas"]["UserInfo"];

    let { children } = $props();

    let user: UserInfo | null = $state(null);
    let loaded = $state(false);

    setContext("user", () => user);

    initAnalytics();

    // https://webkilt.andrew.cmu.edu/layout.html#scripts
    const SCRIPTS = [
        "https://www.cmu.edu/common/standard-v6/js/list.min.js",
        "https://www.cmu.edu/common/standard-v6/js/jquery-1.10.1.min.js",
        "https://www.cmu.edu/common/standard-v6/js/main.6.7.min.js",
        "https://cdn.jsdelivr.net/gh/cmu-web/cmu-web-kilt@0.4.2/js/cmu-app.js",
    ];

    function loadScript(src: string): Promise<void> {
        const { promise, resolve, reject } = Promise.withResolvers<void>();
        const script = document.createElement("script");
        script.src = src;
        script.addEventListener("load", () => resolve());
        script.addEventListener("error", () => reject(new Error(`failed to load ${src}`)));
        document.body.append(script);
        return promise;
    }

    async function start() {
        const { data } = await api.GET("/api/me");
        if (data) {
            user = data;
            identify(data.subject, data.name);
        }
        loaded = true;

        // main.js reads the menu on load, and the user isn't rendered yet
        await tick();
        for (const src of SCRIPTS) {
            await loadScript(src);
        }
    }

    function openMenu(event: MouseEvent) {
        event.preventDefault();
        document.querySelector<HTMLButtonElement>(".roof .menu-btn")?.click();
    }

    start();
</script>

<!-- https://webkilt.andrew.cmu.edu/layout.html#skeleton -->
<header>
    <div class="roof red invert">
        <div>
            <a class="wordmark" href="https://www.cmu.edu/">Carnegie Mellon University</a>
            <!-- https://webkilt.andrew.cmu.edu/accounts.html#account -->
            {#if user}
                <a class="cmu-account cmu-account-open" href="#nav" aria-label="Menu for {user.name}" onclick={openMenu}>{user.name}</a>
            {:else if loaded}
                <a class="cmu-account" href="/auth/login">Sign In</a>
            {/if}
            <button aria-label="toggle menu" aria-pressed="false" class="menu-btn"><b>&#8212;</b><b>&#8212;</b><b>&#8212;</b></button>
        </div>
    </div>
    <div id="sitename">
        <span class="sitetitle"><a href="/">Link Shortener</a></span>
    </div>
</header>

<main id="content">
    {#if loaded}
        {@render children()}
    {/if}
</main>

<footer class="grid column3 invert">
    <div>
        <p>Made with &#x2764;&#xFE0F; by <a href="https://scottylabs.org/">ScottyLabs</a></p>
        <ul>
            <li><a href="https://docs.cmu.lol/">Documentation</a></li>
            <li><a href="/api/scalar">API Reference</a></li>
            <li><a href="https://git.cmu.dev/ScottyLabs/link-shortener">Source&nbsp;Code</a></li>
        </ul>
    </div>
    <div hidden>
        <nav id="nav">
            <ul>
                {#if user}
                    <li class="secondary cmu-account-menu">
                        <!-- svelte-ignore a11y_missing_attribute -->
                        <a class="cmu-account-name">{user.name}</a>
                        <a href="/auth/logout" onclick={resetAnalytics}>Sign Out</a>
                    </li>
                {:else}
                    <li class="secondary cmu-account-menu">
                        <a href="/auth/login">Sign In</a>
                    </li>
                {/if}
            </ul>
        </nav>
    </div>
</footer>

<style>
    :global(html) {
        overscroll-behavior-y: none;
    }

    :global(#container) {
        display: flex;
        flex-direction: column;
        min-height: 100vh;
    }

    main {
        flex: 1;
    }

    :global(#content > div.content:first-child) {
        padding-top: 1em;
    }

    .roof .menu-btn {
        top: 50%;
        margin-top: 0;
        transform: translateY(-50%);
    }

    footer.grid > div {
        padding-top: 0;
        padding-bottom: 0;
    }

    @media (max-width: 767px) {
        :global(:is(.roof, header > div, .grid, .breadcrumbs, .content)) {
            padding-left: 1.25rem;
            padding-right: 1.25rem;
        }

        :global(header:not(.float) #sitename) {
            margin: 0 1.25rem;
        }
    }

    :global(#content :is(input:not([type="checkbox"], [type="radio"]), select)) {
        box-sizing: border-box;
        height: 2.2em;
    }

    :global(#content select) {
        padding: 0 0.5em;
    }
</style>
