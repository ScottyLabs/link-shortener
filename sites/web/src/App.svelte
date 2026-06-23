<script lang="ts">
    import CreateLink from "./lib/components/CreateLink.svelte";
    import LinkList from "./lib/components/LinkList.svelte";

    type UserInfo = { subject: string };

    let user: UserInfo | null = $state(null);
    let loading = $state(true);

    async function checkAuth() {
        try {
            const res = await fetch("/api/me");
            if (res.ok) {
                user = await res.json();
            }
        } finally {
            loading = false;
        }
    }

    checkAuth();
</script>

<main>
    <h1>ScottyLabs Link Shortener</h1>

    {#if loading}
        <p>Loading...</p>
    {:else if user}
        <p>Logged in as {user.subject}</p>
        <a href="/auth/logout">Log out</a>
        <CreateLink onCreated={() => location.reload()} />
        <LinkList />
    {:else}
        <a href="/api/me">Log in with ScottyLabs</a>
    {/if}
</main>
