<script>
    let { data } = $props();
</script>

<h1 class="text-3xl">{data.tournament.name}</h1>
<p class="text-cl">{data.tournament.edition}</p>

<div class="grid auto-cols-max gap-4 mt-4">
    {#each data.teams as team}
        <a href="/teams/{team.id}">
            <div class="p-6 bg-amber-200 aspect-square rounded-lg shadow-sm">
                <h2 class="text-2xl">{team.name}</h2>
                <p class="text-lg">{data.players[team.captain_id].name}</p>
                <p>{data.players[team.partner_id].name}</p>
            </div>
        </a>
    {/each}
</div>
