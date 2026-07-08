<script lang="ts">
    import { api } from "$lib/api/client";
    import type { components } from "$lib/api/schema";
    import CreateLink from "$lib/components/CreateLink.svelte";
    import LinkList from "$lib/components/LinkList.svelte";

    type UserInfo = components["schemas"]["UserInfo"];

    let user: UserInfo | null = $state(null);
    let loading = $state(true);

    async function checkAuth() {
        try {
            const { data } = await api.GET("/api/me");
            if (data) {
                user = data;
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
        <p>Logged in as {user.name}</p>
        <a href="/auth/logout">Log out</a>
        {#if user.can_create}
            <CreateLink onCreated={() => location.reload()} />
        {/if}
        <LinkList isAdmin={user.is_admin} canModify={user.can_create} />
    {:else}
        <a href="/auth/login">Log in with ScottyLabs</a>
    {/if}
</main>
