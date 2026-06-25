export async function load({ fetch, params: {id} }) {
    const response = await fetch(`/api/teams/${id}`);
    const [team, games] = await response.json();

    const [captain, partner] = await Promise.all([
        await fetch(`/api/players/${team.captain_id}`)
            .then(r => r.json()).then(player => player[0]),
        await fetch(`/api/players/${team.partner_id}`)
            .then(r => r.json()).then(player => player[0])
    ]);

    return { team, games, captain, partner };
}
