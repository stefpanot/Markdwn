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

## Piste 1 — Coloration syntaxique des blocs de code

**Intention.** Les blocs de code de l'aperçu portent la coloration de leur
langage. Candidat naturel : `syntect` côté Rust (coloration au moment du
rendu, thèmes embarqués, zéro dépendance JS), avec sortie en classes CSS
plutôt qu'en styles en ligne pour rester sur les tokens du thème clair/sombre.

**À décider au démarrage.** Poids du binaire (quelques Mo de définitions de
langages) vs coloration à la demande dans la webview.

## Piste 2 — Recherche multi-fichiers

**Intention.** Chercher une chaîne dans tous les `.md` du dossier ouvert
(`Ctrl+Shift+F`), résultats avec extrait et ligne, ouverture au bon endroit.
Le scan vit en Rust (parcours déjà borné à 16 niveaux / 5000 fichiers) ; la
surface de saisie réutilise la palette ou un panneau dédié, à trancher en
maquette d'abord.

## Piste 3 — Portage macOS / Linux

**Intention.** La CI construit déjà sur trois plateformes ; le travail est
surtout de la vérification et du polissage : raccourcis `Ctrl` → `Cmd`,
chrome de fenêtre custom vs natif macOS (traffic lights), menu applicatif,
packaging (`.dmg`, `.AppImage`/`.deb`), et la signature/notarisation restant
hors scope pour l'instant. Point de vigilance : il faut une machine de chaque
OS pour tester réellement.

## Piste 4 (optionnelle) — Plan adaptatif sur les très longs documents

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
