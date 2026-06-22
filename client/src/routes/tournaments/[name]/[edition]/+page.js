export async function load({ fetch, params: {name, edition} }) {
    const response = await fetch(`/api/tournaments/${name}/${edition}`);
    const [tournament, teams] = await response.json();

    const ids = teams.flatMap(team => [team.captain_id, team.partner_id]);

    const entries = await Promise.all(ids.map(async id => {
        const response = await fetch(`/api/players/${id}`);
        const [player, _] = await response.json();
        return [id, player];
    }));

    const players = Object.fromEntries(entries);

    return { tournament, teams, players };
}
