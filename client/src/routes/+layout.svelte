<script>
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<nav class="p-4 flex justify-between bg-amber-200">
    <a href="/">Gestruc</a>
    <a href="/tournaments">Tornejos</a>
    <a href="/teams">Equips</a>
    <a href="/games">Partides</a>

    <div class="flex gap-4">
        <p>Usuari</p>
        <a href="/account">account</a>
    </div>
</nav>

<main class="p-8">
    {@render children()}
</main>
