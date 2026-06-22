<script>
    let { data } = $props();
</script>

<h1 class="text-3xl">Tornejos</h1>

<div class="grid auto-cols-max gap-4 mt-4">
    {#each data.tournaments as tournament}
        <a href="/tournaments/{tournament.name}/{tournament.edition}">
            <div class="p-6 bg-amber-200 aspect-square rounded-lg shadow-sm">
                <h2 class="text-2xl">{tournament.name}</h2>
                <p class="text-xl">{tournament.edition}</p>
            </div>
        </a>
    {/each}
</div>
