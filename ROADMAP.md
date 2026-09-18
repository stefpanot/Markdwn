# Feuille de route

Markdwn est un éditeur/lecteur Markdown natif, en développement actif. Ce
fichier fixe les pistes au-delà du quotidien : ce qui est livré récemment, ce
qui est prévu, dans quel ordre, et ce qui est explicitement hors scope.

Les principes qui guident chaque piste restent ceux du projet : trois modes
distincts (Lecture, Split, Zen) ; tout ce qui touche au disque et au parsing
vit en Rust ; la mise en page reste dans la webview.

## Livré récemment

- Palette de commandes (`Ctrl+K`) : recherche floue fichiers + commandes.
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

## Piste 2 — Recherche et remplacement (`Ctrl+H`)

**Intention.** Chercher et remplacer dans le document courant, avec
occurrences suivante/précédente. Le moteur vit en Rust et servira de socle à
la recherche multi-fichiers ; la surface de saisie réutilise la palette ou un
panneau dédié, à trancher en maquette d'abord.

## Piste 3 — Recherche multi-fichiers

**Intention.** Chercher une chaîne dans tous les `.md` du dossier ouvert
(`Ctrl+Shift+F`), résultats avec extrait et ligne, ouverture au bon endroit.
Le scan vit en Rust (parcours déjà borné à 16 niveaux / 5000 fichiers) ; le
moteur de la Piste 2 est réutilisé tel quel.

## Piste 4 — Portage macOS / Linux

**Intention.** La CI construit déjà sur trois plateformes ; le travail est
surtout de la vérification et du polissage : raccourcis `Ctrl` → `Cmd`,
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

Analyse (sept. 2026) de captures de Markdown Monster : garder l'esprit, pas la
copie. Ce qui suit trace les choix pour ne pas les re-débattre.

**Retenu** (intégré aux pistes ci-dessus) :

- Badges langage + bouton copier sur les blocs de code → Piste 1 (coloration
  déjà livrée).
- Find in Files → Pistes 2 / 3.
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
