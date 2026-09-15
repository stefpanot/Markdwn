# MarkdownEdit

Éditeur **et lecteur** Markdown natif pour Windows (puis macOS et Linux).
Tauri 2 · Svelte 5 · CodeMirror 6 · parsing en Rust.

## La thèse : trois postures

La lecture compte autant que l'écriture. Ce ne sont pas les mêmes besoins de
mise en page, donc la densité se règle **par mode**, jamais globalement.

| Mode        | Raccourci | Ce qu'il optimise                                              |
| ----------- | --------- | -------------------------------------------------------------- |
| **Lecture** | `Ctrl+1`  | le document plein cadre, colonne de 712 px, sommaire, historique |
| **Split**   | `Ctrl+2`  | écrire en vérifiant le rendu, scroll synchronisé                |
| **Zen**     | `Ctrl+3`  | écrire seul, tout le chrome s'efface (`Esc` pour sortir)        |

Le sélecteur de mode est le cœur de l'interface, pas un bouton de barre d'outils.

## Raccourcis

| Raccourci   | Action                                                        |
| ----------- | ------------------------------------------------------------- |
| `Ctrl+1/2/3`| Lecture / Split / Zen (`Esc` sort du Zen)                     |
| `Ctrl+B`    | Masquer / afficher la barre de dossiers — **dans tous les modes** |
| `A` `A`     | Taille du texte en Lecture (15 / 17 / 19 / 21 px)             |
| `Alt+↑` `Alt+↓` | Fichier précédent / suivant du dossier, dans l'ordre de l'arbre |
| `Ctrl+O`    | Ouvrir un fichier                                             |
| `Ctrl+S`    | Enregistrer                                                   |
| `Ctrl+N`    | Nouveau document                                              |
| `Ctrl+W`    | Fermer l'onglet                                               |
| `Ctrl+,`    | Paramètres                                                    |

Clic droit sur un onglet : **Fermer**, **Fermer les autres**, **Fermer tout**.
Fermer un document modifié demande confirmation.

Clic sur le **logo** en haut à gauche : menu applicatif (façon Zed).

### Déplacer et redimensionner la fenêtre

`decorations: false` fait gagner 40 px de hauteur, mais coûte les comportements
natifs de la zone non-cliente. Deux pièges, tous deux traités :

**Le déplacement.** `data-tauri-drag-region` ne s'active que si l'élément
**cible** du clic porte l'attribut — pas un ancêtre. Un conteneur d'onglets en
`flex: 1` couvre alors toute la barre et ne laisse que des slivers saisissables.

L'attribut est donc posé sur les parents *aussi* (`.titlebar`, `.tabs`), ce qui
est sans risque : puisque Tauri teste la cible, un parent porteur rend
saisissable tout ce qu'aucun enfant ne couvre — marges, interstices entre
onglets, bandes au-dessus et en dessous — sans rendre les enfants moins
cliquables. Les onglets sont dimensionnés à leur contenu, l'espace restant est
une zone de préhension d'au moins 90 px, et les espaces vides des barres de mode
en sont aussi. Le double-clic dessus agrandit la fenêtre.

**Le redimensionnement.** Les bordures natives disparaissent avec
`decorations: false`. `ResizeEdges.svelte` les rétablit : huit poignées
invisibles de 5 px (9 px aux coins) qui appellent `startResizeDragging`, neutres
quand la fenêtre est agrandie.

**Décision arrêtée le 2026-09-10 : pas de Win32.** Deux comportements Windows
restent donc non couverts — l'**Aero Snap** en glissant vers un bord, et les
**Snap Layouts** au survol du bouton Agrandir. Les récupérer exigerait de garder
le cadre natif et d'intercepter `WM_NCCALCSIZE` / `WM_NCHITTEST` via un
sous-classement du WNDPROC (`windows-rs`). Écarté : trop de code non trivial et
de risque pour le gain. **Ne pas reproposer sans demande explicite.**

Contournement sans coût : les raccourcis du shell continuent de fonctionner sur
une fenêtre sans décorations — `Win+←` / `Win+→` pour ancrer, `Win+↑` pour
agrandir. Ça couvre l'essentiel de l'usage réel du snap.

## Distribuer

`.\dev.ps1 build` produit trois choses dans `src-tauri/target/release/` :

| Artefact | Taille | Usage |
| --- | --- | --- |
| `MarkdownEdit.exe` | 5,5 Mo | exécutable autonome, aucune installation |
| `bundle/msi/MarkdownEdit_0.1.0_x64_en-US.msi` | 3,1 Mo | installeur MSI (déploiement d'entreprise, GPO) |
| `bundle/nsis/MarkdownEdit_0.1.0_x64-setup.exe` | 2,5 Mo | installeur classique |

L'exécutable de `target/debug/` ne fonctionne **pas** en autonome : il va chercher
le serveur Vite sur `localhost:1420`. Seul le build release embarque le front.

> [!NOTE]
> Les binaires ne sont pas signés. Sur une autre machine, SmartScreen affichera
> un avertissement « Éditeur inconnu ». Il faut un certificat de signature de
> code pour l'éviter.

### Empreinte mémoire mesurée

Au démarrage, document d'accueil ouvert, mode Split :

| | |
| --- | --- |
| Processus principal (Rust) | 27 Mo |
| Ses 6 processus WebView2 | ~170 Mo de mémoire privée |
| **Total réellement imputable** | **~194 Mo** |

Le working set cumulé affiche ~423 Mo, mais il inclut des pages partagées avec
les autres applications WebView2 de la machine — c'est le chiffre à ne pas citer.

C'est le coût honnête du choix Tauri : une GUI Rust native tiendrait sous 50 Mo.
En échange, l'aperçu Markdown est du vrai HTML et le démarrage reste rapide, ce
qui était le critère.

## Espace disque

Un dossier de projet à ~5 Go est normal pour Tauri, et ce n'est **pas** dans le
dépôt : `src-tauri/target/` en représente ~98 % et est ignoré par Git.

Répartition typique après un build debug **et** un build release :

| | Mo | Quoi |
| --- | --- | --- |
| `target/debug` | ~3 400 | dont 1 900 de dépendances compilées |
| `target/release` | ~1 560 | ne sert qu'à produire le livrable |
| `node_modules` | ~110 | |

Par extension : `.rlib` 1 833 · `.lib` 1 060 · `.rmeta` 688 · `.pdb` 666.
Autrement dit **les bibliothèques compilées des ~380 crates de Tauri**, pas des
infos de debug — le réglage ci-dessous fait déjà son travail, les PDB ne pèsent
que 666 Mo sur 5 Go.

```toml
[profile.dev.package."*"]
debug = false
```

Il coupe les infos de debug des **dépendances seulement** : notre crate reste
entièrement débogable, backtraces comprises. Retirer le bloc s'il faut un jour
entrer dans le code de Tauri.

### Routine de nettoyage

`clean.ps1`, trois niveaux, avec `-DryRun` pour simuler sans rien supprimer :

```powershell
.\clean.ps1                 # sûr : ~370 Mo, prochain build rapide
.\clean.ps1 -Release        # ~3 500 Mo, livrables recopiés dans dist\
.\clean.ps1 -All            # tout target\, prochain build complet (~3 min)
```

`-Release` est le meilleur rapport gain/inconvénient : l'arbre release ne sert
qu'à fabriquer un livrable, et le script **recopie d'abord** le `.exe` autonome
et les deux installeurs dans `dist\` avant de le jeter. Ajouter
`-IncludeNodeModules` à `-All` vide aussi `node_modules` (`npm install` requis
ensuite, donc du réseau).

Le script refuse de tourner si un processus verrouille ce qu'il s'apprête à
supprimer — et seulement dans ce cas : un `cargo`/`rustc` en cours, le serveur
Vite **de ce projet**, ou l'app en cours d'exécution quand `-All` viserait son
binaire. Un `node` sans rapport sur la machine ne le bloque pas.

## Prérequis

- **Rust** (`winget install --id Rustlang.Rustup`)
- **Node 24** — géré par [mise](https://mise.jdx.dev) via `mise.toml`
- **MSVC + SDK Windows** — charge de travail « Développement Desktop en C++ »
- **WebView2** — déjà présent sur Windows 11

> [!IMPORTANT]
> Visual Studio 2026 n'écrit plus la clé de registre
> `HKLM\SOFTWARE\Microsoft\VisualStudio\SxS\VS7`, sur laquelle repose une partie
> de l'auto-détection MSVC de `rustc`. Résultat : `link.exe` est installé mais
> cargo ne le trouve pas, et le build échoue sur
> `error: linker 'link.exe' not found`.
>
> `dev.ps1` charge l'environnement MSVC avant chaque commande — utilise-le
> plutôt que `npm run tauri dev` directement.

## Commandes

```powershell
.\dev.ps1          # lance l'app en développement
.\dev.ps1 build    # produit l'installeur
.\dev.ps1 test     # tests Rust
.\dev.ps1 version  # version de l'app, et vérification de la source unique
.\clean.ps1        # récupère l'espace disque (voir « Routine de nettoyage »)
```

```powershell
mise exec -- npm run check   # types Svelte + TS
```

## Architecture

**La règle : tout ce qui touche au disque et au parsing descend en Rust ; tout
ce qui touche à la mise en page reste dans la webview.**

```
src-tauri/src/
  markdown.rs   rendu Markdown -> HTML + sommaire + compteurs
  lib.rs        commandes Tauri : render_markdown, read_document,
                write_document, list_dir
src/lib/
  api.ts             enveloppes typées autour d'invoke
  state.svelte.ts    état applicatif (runes Svelte 5)
  tokens.css         système de design, thèmes clair et sombre
  editor-theme.ts    thème CodeMirror, lit les tokens CSS
  components/        chrome, éditeur, aperçu
design/
  *.dc.html     maquettes de référence — la source de vérité visuelle
```

### La cartographie source

`markdown.rs` fait **une seule passe** de parsing et insère un marqueur
`<span class="srcmap" data-line="N">` avant chaque bloc de premier niveau.

Ce choix est délibéré : rendre chaque bloc séparément donnerait aussi son
offset, mais casserait les définitions de liens par référence
(`[a]: https://…`) placées ailleurs dans le fichier. Un test verrouille ce
comportement.

Ces marqueurs donnent presque gratuitement :

- le **scroll synchronisé** bidirectionnel (verrou de direction côté front pour
  éviter le ping-pong) ;
- le **sommaire** cliquable, avec la section courante suivie ;
- le **saut** depuis le plan vers la bonne ligne de la source.

## Configuration

Persistée dans `%APPDATA%\com.spanot.markdownedit\config.json`, **hors du
dossier d'installation** : une mise à jour ne l'écrase pas. Le panneau
**Paramètres** (`Ctrl+,`, ou le menu du logo) sait révéler le fichier.

Réglages mémorisés : thème, mode courant, taille et largeur de lecture,
visibilité de la barre de dossiers, scroll synchronisé, dernier dossier ouvert
et s'il faut le rouvrir. La **taille et la position de la fenêtre** passent par
le plugin officiel `tauri-plugin-window-state`, dans son propre fichier — inutile
de refaire ce travail.

Le mode au démarrage est simplement **le dernier utilisé** : pas de réglage
séparé « mode par défaut » à maintenir en parallèle de l'état réel.

### Les trois garanties, et comment elles sont tenues

`src-tauri/src/config.rs`, verrouillé par 8 tests.

**Schéma versionné.** `SCHEMA_VERSION` et une fonction `migrate` qui travaille
sur le **JSON brut**, pas sur la structure `Config` — une migration doit pouvoir
lire des champs qui n'existent plus dans le code actuel. Les migrations
suivantes s'ajoutent en `if from < N`.

**Clés inconnues préservées.** À l'enregistrement, le fichier est relu et nos
champs sont fusionnés dedans. Sans ça, lancer une version plus récente puis
revenir en arrière amputerait silencieusement les réglages que l'ancienne
version ne connaît pas. Vérifié en test *et* de bout en bout dans l'app.

**Rien n'est jamais perdu.** Un fichier illisible ou mal typé est renommé en
`config.json.bak` et l'app repart sur les défauts avec un message. Un fichier
absent n'est pas une anomalie : c'est un premier lancement, sans avertissement.

Deux détails d'implémentation qui comptent :

- **Rust possède `schemaVersion`** — il le réécrit à l'enregistrement quoi que
  le front envoie. Le front n'a pas à connaître la version du format.
- **Le garde `hydrated`** côté front : tant que la configuration n'est pas
  chargée, l'effet d'enregistrement ne tire pas. Sans lui, le premier rendu
  écraserait le fichier avec les valeurs par défaut.

### Menu applicatif

Un clic sur le logo, en haut à gauche, ouvre le menu — actions fichier, modes,
bascules d'affichage, paramètres. Les entrées d'état portent une coche et les
raccourcis sont affichés. Le mode Zen n'a pas de titlebar : `Ctrl+,` y reste le
chemin vers les paramètres.

### Version de l'application

**Source unique : `src-tauri/Cargo.toml`.** Les deux duplicatas ont été
supprimés plutôt que synchronisés :

- `tauri.conf.json` n'a plus de champ `version` — Tauri retombe alors sur celle
  de `Cargo.toml`. Vérifié jusque dans les métadonnées de ressource Windows du
  binaire (`FileVersion: 0.1.0`).
- `package.json` n'en a plus non plus : le paquet est `private`, sa version ne
  sert à rien ici. `npm run check` fonctionne sans.

L'app lit sa propre version par la commande `app_version` et l'affiche dans le
panneau de paramètres.

```powershell
.\dev.ps1 version          # affiche la version et vérifie la source unique
.\dev.ps1 version 0.2.0    # monte la version (semver validé)
```

`Assert-SingleVersionSource` **échoue** si un champ `version` réapparaît dans
`tauri.conf.json` ou `package.json`, et `.\dev.ps1 build` l'appelle avant de
construire : un binaire dont la version a dérivé ne peut pas être livré. Les
deux cas de dérive sont testés.

C'est le prérequis du plugin `updater`, qui compare la version installée à celle
annoncée par le serveur de mise à jour : deux sources divergentes y produiraient
des mises à jour fantômes ou manquantes.

## État actuel

Fonctionnel : la **configuration persistée**, le **panneau de paramètres**, le
**menu applicatif**, l'écran d'accueil, les trois modes, l'édition CodeMirror avec coloration Markdown,
l'aperçu rendu par Rust, le sommaire, l'arborescence de dossier masquable dans
tous les modes, la navigation fichier suivant/précédent, **le suivi des liens
relatifs entre documents**, **deux largeurs de lecture**, ouvrir / enregistrer,
le scroll synchronisé, les thèmes clair et sombre, le formatage (gras, italique,
lien, code, liste, citation), les onglets multi-documents avec menu contextuel
et confirmation avant de jeter des modifications.

### Aucun document ouvert

Fermer le dernier onglet ramène à un **écran d'accueil**, sans onglet fantôme :
actions principales, raccourcis, et le nom du dossier ouvert s'il y en a un. La
barre de dossiers reste disponible pour choisir un fichier.

Deux conséquences dans le code :

- `app.docs` **peut être vide**, donc `app.active` est de type
  `OpenDoc | undefined`. Tout ce qui le lit doit gérer l'absence — le
  vérificateur de types l'impose désormais.
- Chaque document ouvert porte un **id stable**. Suivre l'index serait faux :
  les index sont réutilisés dès qu'un onglet se ferme, ce qui laissait le texte
  du document précédent affiché après « fermer tout », et rechargeait
  inutilement le document courant quand on fermait un onglet à sa gauche
  (perdant curseur et historique d'annulation).

Pas de « fichiers récents » sur cet écran : ça demande la configuration
persistée, qui n'existe pas encore. Mieux vaut ne rien promettre.

Si un dossier est ouvert mais la barre masquée, l'encart contextuel devient un
**bouton** qui la réaffiche — plutôt que de conseiller « choisis un fichier dans
la barre latérale », conseil impossible à suivre dans cet état.

### Où vit le bouton de la barre de dossiers

Dans la **titlebar**, juste à droite du logo, et nulle part ailleurs.

C'est délibéré : la titlebar est le seul chrome présent dans tous les modes **et**
sur l'écran d'accueil. Quand le bouton vivait dans la barre d'outils et la barre
de lecture, replier la barre puis fermer tous les fichiers le rendait
inatteignable à la souris — seul `Ctrl+B` sortait de l'impasse. Le mode Zen, qui
n'a pas de titlebar, porte le même bouton dans son groupe flottant.

Règle générale qui en découle : **toute bascule qui peut masquer son propre
moyen de réapparaître doit vivre dans un chrome permanent.**

### Suivre les liens

Un clic sur un lien de l'aperçu ne navigue jamais la webview. La page décide :

| Lien | Comportement |
| --- | --- |
| `./local-setup.md`, `../architecture/glossary.md` | ouvert dans un onglet, ajouté à l'historique |
| `dependency-map.md#port-registry` | ouvert **puis** défilé jusqu'à l'ancre |
| `#scope-caveats` | défilement dans le document courant |
| `./local-setup` (sans extension) | l'extension `.md` est tentée |
| `./mon%20fichier.md` | le percent-encoding est décodé |
| `./diagramme.pdf`, `./photo.png` | confié à l'application système |
| `https://…`, `mailto:…` | ouvert dans le navigateur ou le client mail |
| cible absente | message d'erreur avec le chemin résolu, rien ne s'ouvre |

La résolution des `..` est **lexicale** et vit en Rust : `canonicalize` exigerait
que la cible existe et renverrait des chemins préfixés `\\?\` sous Windows.

Les titres portent une ancre dérivée de leur texte, dédupliquée
(`notes`, `notes-2`), ce qui rend les liens profonds fiables.

À faire, par ordre d'importance pour la lecture :

- [ ] **Configuration persistée et versions** — voir la section dédiée ci-dessus
- [ ] **Images relatives dans l'aperçu** — `<img src="./schema.png">` ne charge
      pas : il faut activer `assetProtocol` et réécrire les `src`. Demande de
      trancher la portée (`scope`) autorisée, puisque l'utilisateur ouvre des
      dossiers arbitraires — décision de sécurité, pas juste de la plomberie
- [ ] **Command palette** (`Ctrl+P`) — recherche de fichiers et commandes
- [ ] Recherche multi-fichiers via la crate `grep` (le moteur de ripgrep)
- [ ] Coloration syntaxique des blocs de code dans l'aperçu (`syntect` en Rust)
- [ ] Effet Mica réel : demande `transparent: true` et un fond de webview
      transparent ; pour l'instant le dégradé est peint par nous
- ~~Aero Snap et Snap Layouts Windows 11~~ — **écarté** le 2026-09-10, voir
  « Déplacer et redimensionner la fenêtre »
- [ ] macOS et Linux via runners GitHub Actions (pas de cross-compilation
      possible depuis Windows)
- [ ] Preview dans l'explorateur Windows (`IPreviewHandler`, DLL COM) — phase 3
