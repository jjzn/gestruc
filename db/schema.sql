PRAGMA foreign_keys = ON;

CREATE TABLE players (
	id TEXT PRIMARY KEY,
	email TEXT NOT NULL UNIQUE,
	password TEXT NOT NULL,
	name TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE tournaments (
	name TEXT NOT NULL,
	edition TEXT NOT NULL,
	PRIMARY KEY (name, edition)
) STRICT;

CREATE TABLE teams (
	id TEXT PRIMARY KEY,
	name TEXT NOT NULL,
	captainId TEXT NOT NULL,
	partnerId TEXT NOT NULL,
	tournamentName TEXT NOT NULL,
	tournamentEdition TEXT NOT NULL,
	FOREIGN KEY (captainId) REFERENCES players(id),
	FOREIGN KEY (partnerId) REFERENCES players(id),
	FOREIGN KEY (tournamentName, tournamentEdition) REFERENCES tournaments(name, edition)
) STRICT;

CREATE TABLE games (
	id TEXT PRIMARY KEY,
	date INTEGER NOT NULL,
	scoresA INTEGER NOT NULL,
	scoresB INTEGER NOT NULL,
	acceptedByA INTEGER,
	acceptedByB INTEGER,
	teamA TEXT NOT NULL,
	teamB TEXT NOT NULL,
	FOREIGN KEY (teamA) REFERENCES teams(id),
	FOREIGN KEY (teamB) REFERENCES teams(id)
) STRICT;

CREATE TABLE sessions (
	id TEXT PRIMARY KEY,
	playerId TEXT NOT NULL,
	expires INTEGER NOT NULL,
	FOREIGN KEY (playerId) REFERENCES players(id)
) STRICT;
