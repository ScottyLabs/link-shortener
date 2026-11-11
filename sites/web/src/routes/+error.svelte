<script lang="ts">
    import { page } from "$app/state";
</script>

<!-- https://webkilt.andrew.cmu.edu/messages.html#errors -->
<div class="content">
    <div>
        {#if page.status === 404}
            <h1>Not Found</h1>
            <p class="cmu-lead">This link doesn't exist or isn't active.</p>
        {:else}
            <h1>Something Went Wrong</h1>
            <p class="cmu-lead">Try again in a few minutes.</p>
        {/if}
        <div class="cmu-cta-row">
            <a class="btn" href="/">Go to the Home Page</a>
        </div>
        <p class="cmu-muted">Error {page.status}</p>
    </div>
</div>
