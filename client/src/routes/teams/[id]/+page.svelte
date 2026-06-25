<script>
    let { data } = $props();
</script>

<h1 class="text-3xl">{data.team.name}</h1>
<p class="text-lg"><a href="/players/{data.captain.id}">{data.captain.name}</a></p>
<p class="text-md"><a href="/players/{data.partner.id}">{data.partner.name}</a></p>
<p class="text-md"><a href="/tournament/{data.team.tournament_name}/{data.team.tournament_edition}">{data.team.tournament_name}, {data.team.tournament_edition}</a></p>

<div class="grid auto-cols-max gap-4 mt-4">
    {#each data.games as game}
        <a href="/games/{game.id}">
            <div class="p-6 bg-amber-200 aspect-square rounded-lg shadow-sm">
                <h2 class="text-2xl">{game.team_names[0]} vs {game.team_names[1]}</h2>
                <p>{game.team_names[0]}: {game.scores[0].join(", ")}</p>
                <p>{game.team_names[1]}: {game.scores[1].join(", ")}</p>
            </div>
        </a>
    {/each}
</div>
