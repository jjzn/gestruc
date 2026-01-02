PRAGMA foreign_keys = ON;

INSERT INTO players VALUES('user01','jjzn@protonmail.com','test123','Joshua Zien');
INSERT INTO players VALUES('user02','toni@fuentes.com','test123','Toni Fuentes');
INSERT INTO players VALUES('user03','03@usuaris.com','test123','Usuari 03');
INSERT INTO players VALUES('user04','04@usuaris.com','test123','Usuari 04');
INSERT INTO players VALUES('user05','05@usuaris.com','test123','Usuari 05');
INSERT INTO players VALUES('user06','06@usuaris.com','test123','Usuari 06');

INSERT INTO tournaments VALUES('Lliga UIB',2025);

INSERT INTO teams VALUES('team01','Envit de Fenwick','user02','user01','Lliga UIB',2025);
INSERT INTO teams VALUES('team02','Retrucs','user03','user04','Lliga UIB',2025);
INSERT INTO teams VALUES('team03','AmoMadona','user05','user06','Lliga UIB',2025);

INSERT INTO games VALUES('game01',1767294000,1445400,1579027,NULL,1767300300,'team01','team02');
INSERT INTO games VALUES('game02',1767294000,1445400,1579027,NULL,1767300300,'team01','team03');
INSERT INTO games VALUES('game03',1767294000,1445400,1579027,NULL,1767300300,'team03','team02');
