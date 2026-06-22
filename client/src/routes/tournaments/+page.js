export async function load({ fetch }) {
    const response = await fetch("/api/tournaments");
    const tournaments = await response.json();

    return { tournaments };
}
