# DTV Ingest

Petite appli qui tourne en fond sur ton PC et envoie à [DTV](https://dtv.gg) les parties
Deadlock que tu joues. DTV récupère ainsi tout seul les parties de tournoi (salons privés)
que ni deadlock-api ni notre bot Steam ne voient.

C'est un fork de [deadlock-api-ingest](https://github.com/deadlock-api/deadlock-api-ingest)
(licence MIT) : tout ce que fait l'original, il le fait aussi. Les parties partent toujours
chez deadlock-api, **et** chez DTV.

## Ce qu'elle envoie

Pour chaque partie : son identifiant et ses « salts », les clés qui permettent de
télécharger sa metadata (joueurs, héros, score) sur les serveurs de Valve. Rien d'autre :
ni fichier, ni identifiant de connexion, ni donnée personnelle.

Elle les trouve à trois endroits :

- **le cache HTTP de Steam** (`Steam/appcache/httpcache/`), quand le jeu télécharge une partie ;
- **la mémoire du jeu**, en lecture seule, pendant que Deadlock tourne (désactivable avec `--no-memory`) ;
- **le coordinateur de jeu de Valve**, avec ta session Steam, pour ton propre historique
  (40 requêtes par jour au plus, jamais pendant que tu joues ; désactivable avec `--no-gc`).

## Installation

### Windows

Télécharge [`dtv-ingest-setup.exe`](https://github.com/DTV-GG/dtv-ingest/releases/latest/download/dtv-ingest-setup.exe)
et lance-le. Pas besoin des droits administrateur. L'appli s'installe dans
`%LOCALAPPDATA%\dtv-ingest\`, démarre avec Windows et ouvre sa fenêtre d'état.

> Si Windows SmartScreen bloque le fichier : **Informations complémentaires → Exécuter quand même**.

### Linux

```bash
curl -fsSL https://raw.githubusercontent.com/DTV-GG/dtv-ingest/master/install-linux.sh | bash
```

Sans curl (Ubuntu de base n'a que wget) :

```bash
wget -O- https://raw.githubusercontent.com/DTV-GG/dtv-ingest/master/install-linux.sh | bash
```

Lance la commande avec ton utilisateur, sans `sudo`.

Pour lire la mémoire du jeu, l'installeur propose un petit utilitaire,
`dtv-ingest-memgrant`, qui demande ton mot de passe une fois. Sans lui, lance plutôt le jeu
par l'option de lancement Steam `~/.local/bin/dtv-ingest -- %command%`.

## Relier l'appli à ton compte DTV

1. Sur [dtv.gg](https://dtv.gg), ouvre ton profil puis **App DTV Ingest** et génère ton jeton.
2. Colle-le dans la fenêtre de l'appli (**Link**), ou en ligne de commande :

   ```bash
   dtv-ingest --dtv-token dtvi_xxxxxxxx
   ```

Les parties déjà envoyées à deadlock-api partent alors aussi chez DTV. `--dtv-logout` délie
l'appli. Le jeton se révoque depuis ton profil DTV.

Variables utiles : `DTV_INGEST_TOKEN` (jeton sans fichier), `DTV_INGEST_URL`
(autre instance, par défaut `https://api.dtv.gg/api/ingest`).

## Si tu as déjà deadlock-api-ingest

Désinstalle-le : DTV Ingest fait la même chose et envoie aussi à deadlock-api. Avec les deux,
ton compte Steam dépenserait deux fois son quota auprès du coordinateur de jeu.

## Désinstaller

- **Windows** : Paramètres → Applications → Applications installées → DTV Ingest.
- **Linux** : `~/.local/share/dtv-ingest/uninstall-linux.sh`

## Développement

```bash
cargo build
cargo clippy --all-targets -- -D warnings
cargo test
```

Pour suivre l'original : `git fetch upstream && git merge upstream/master`, puis refaire le
renommage sur les nouveaux fichiers. Le code propre à DTV est dans `src/dtv.rs`, plus son
appel dans `src/utils.rs`, `--dtv-token` dans `src/main.rs` et la carte DTV dans `src/gui/mod.rs`.

## Licence

MIT, comme [deadlock-api-ingest](https://github.com/deadlock-api/deadlock-api-ingest), dont
ce dépôt est un fork.
