<script>
    const games = ["hola", "mon"];
</script>

<h1 class="text-3xl">Partides més recents</h1>

<ul>
    {#each games as game}
        <li>{game}</li>
    {/each}
</ul>
