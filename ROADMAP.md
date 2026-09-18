# Feuille de route

Markdwn est un éditeur/lecteur Markdown natif, en développement actif. Ce
fichier fixe les pistes au-delà du quotidien : ce qui est livré récemment, ce
qui est prévu, dans quel ordre, et ce qui est explicitement hors scope.

Les principes qui guident chaque piste restent ceux du projet : trois modes
distincts (Lecture, Split, Zen) ; tout ce qui touche au disque et au parsing
vit en Rust ; la mise en page reste dans la webview.

## Livré récemment

- « Enregistrer sous… » (`Ctrl+Shift+S`) : copie du document vers un autre
  chemin via le dialogue natif ; l'onglet suit la copie, l'original reste
  intact. Pur travail de webview (`write_document` existant, aucun Rust
  nouveau).

- Recherche et remplacement (`Ctrl+H`) : moteur en Rust (`search.rs`, socle de
  la Piste 2), offsets UTF-16 natifs pour CodeMirror, options casse / mot
  entier. Surface tranchée : panneau dédié sous la barre d'outils — la
  palette, transitoire par nature, se prête mal à la navigation occurrence
  par occurrence. Suivant / précédent (Entrée / Maj+Entrée), remplacer,
  remplacer tout, Échap referme.

- Palette de commandes (`Ctrl+K`) : recherche floue fichiers + commandes.
- Fixs raccourcis : Ctrl+K rendu à la palette (la keymap CodeMirror le
  liait à « supprimer jusqu'en fin de ligne », héritage Emacs) ; Ctrl+R,
  Ctrl+Shift+R, F5 et Ctrl+F5 neutralisés — WebView2 les traitait comme un
  navigateur et recharger la page faisait perdre l'état en mémoire,
  documents non enregistrés compris. Voir la Piste 3 pour le fond du sujet.
- Association « ouvrir avec » Windows : l'installeur NSIS enregistre les
  `.md` / `.markdown` (fileAssociations), l'app reçoit le chemin passé en
  ligne de commande (`consume_initial_file`) et le plugin single-instance
  déroute les double-clics suivants vers l'instance ouverte (événement
  `open-file`). L'installeur demande « pour moi uniquement » ou « pour tous
  les utilisateurs » (`installMode: both`, élévation admin seulement dans
  le second cas).
- Images relatives dans l'aperçu, y compris `<img>` en HTML brut.
- Plan du document : lisible sur les très longs documents, affiché en un seul
  exemplaire par mode.
- Onglets scrollables quand ils débordent ; navigation regroupée et stable.
- Coloration syntaxique des blocs de code (`syntect` en Rust, classes CSS —
  le thème reste une affaire de feuille de style).

## Piste 1 — Badges langage et copie sur les blocs de code

**Intention.** Au-delà de la coloration (livrée), chaque bloc de code de
l'aperçu affiche son langage et un bouton « copier ». Pur travail de webview,
aucun impact sur la couche Rust.

## Piste 2 — Recherche multi-fichiers

**Intention.** Chercher une chaîne dans tous les `.md` du dossier ouvert
(`Ctrl+Shift+F`), résultats avec extrait et ligne, ouverture au bon endroit.
Le scan vit en Rust (parcours déjà borné à 16 niveaux / 5000 fichiers) ; le
moteur `search.rs` livré avec la recherche est réutilisé tel quel.

## Piste 3 — Raccourcis : conflits, accélérateurs webview et mapping par OS

**Intention.** Un audit et une centralisation. Trois problèmes connus, par
ordre de gravité :

1. **Accélérateurs de la webview.** WebView2 (Windows) garde les raccourcis
   navigateur : Ctrl+R / F5 rechargent la page et font perdre l'état en
   mémoire. Bouché en JS pour les touches de rechargement ; le vrai fix vit
   côté Rust (`AreBrowserAcceleratorKeysEnabled` à false sur WebView2,
   équivalent à trouver pour WKWebView) pour couvrir aussi Ctrl+P, Ctrl+F,
   F12, le zoom… — et à recouper avec le portage macOS (Piste 4), où
   Cmd+R posera exactement le même problème.
2. **Conflits avec la keymap CodeMirror.** Son `defaultKeymap` Emacs lie
   Ctrl+K à « supprimer jusqu'en fin de ligne » (bouché en avalant `Mod-k`
   côté éditeur). L'audit conflit app ↔ extensions CM doit être refait à
   chaque nouveau raccourci des deux côtés.
3. **Mapping par OS.** Un seul point de vérité : table raccourci → action
   dont le modifieur (Ctrl vs Cmd) et le libellé affiché (`Ctrl+S` vs
   `⌘S`) sont dérivés de la plateforme. Prérequis du portage macOS, qui
   réutilisera le même travail. Aujourd'hui tout est en dur : handler
   unique dans `+page.svelte`, libellés en dur dans les menus.

## Piste 4 — Portage macOS / Linux

**Intention.** La CI construit déjà sur trois plateformes ; le travail est
surtout de la vérification et du polissage : raccourcis `Ctrl` → `Cmd` (le
fond du travail est la Piste 3, qui rend cette conversion quasi gratuite),
chrome de fenêtre custom vs natif macOS (traffic lights), menu applicatif,
packaging (`.dmg`, `.AppImage`/`.deb`), et la signature/notarisation restant
hors scope pour l'instant. Point de vigilance : il faut une machine de chaque
OS pour tester réellement.

## Piste 5 — Sauvegarde auto, backups et barre de statut

**Intention.** Trois petites fonctions d'éditeur « sérieux », toutes bon
marché côté Rust : sauvegarde automatique (avec écriture atomique), copies
de backup à côté du fichier, et une barre de statut affichant mots, lignes,
position `Ln, Col`, fin de ligne et encodage. La barre de statut vit dans la
webview mais ses compteurs viennent de Rust.

## Piste 6 (optionnelle) — Plan adaptatif sur les très longs documents

**Intention.** Au-delà de ~25 titres, n'afficher par défaut que les deux
premiers niveaux présents, avec un bouton « tout afficher ». À faire seulement
si l'usage le confirme : la liste complète scrollable reste honnête.

## Hors scope (décidé)

- **Mode navigateur comme usage supporté.** `npm run dev` seul reste un mode
  dégradé pour travailler l'UI ; l'accès disque limité du web viderait
  l'outil de son intérêt.
- **Extension VS Code.** Les webviews VS Code imposent leur propre sandbox et
  une réécriture de la couche disque ; seul le cœur Rust serait réutilisable,
  ce qui est un autre projet.
- **File System Access API** pour rendre le navigateur fonctionnel (voir
  premier point).
- **Système d'extensions / add-ins.** C'est un écosystème à part entière
  (API stable, sandbox, distribution), pas une feature : un autre projet.

## Inspirations — analyse d'un éditeur de référence

Analyse (sept. 2026) de captures d'un éditeur de Markdown : garder l'esprit, pas la
copie. Ce qui suit trace les choix pour ne pas les re-débattre.

**Retenu** (intégré aux pistes ci-dessus) :

- Badges langage + bouton copier sur les blocs de code → Piste 1 (coloration
  déjà livrée).
- Find in Files → recherche/remplacement livré, multi-fichiers en Piste 2.
- Sauvegarde auto + backup, barre de statut → Piste 5.
- Distraction Free / modes d'affichage → déjà couvert par les modes
  Lecture / Split / Zen existants.
- Plan du document → déjà livré.

**Écarté, avec raison** :

- **Add-ins / extensions** : écosystème complet, voir Hors scope.
- **Weblog publishing** : cas d'usage de niche, éloigné du cœur (fichiers
  locaux).
- **Add-ins IA** (génération d'images, résumé de sélection) : dépendance à
  des services externes, contraire au caractère natif et autonome de
  l'outil.
- **Speak / synthèse vocale** : niche, coût d'intégration disproportionné.
- **Correcteur orthographique** : non retenu pour l'instant — dictionnaires
  lourds et multilingues ; à réévaluer si la demande utilisateur existe.
