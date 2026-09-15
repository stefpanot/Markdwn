<div align="center">

<img src="assets/logo-lockup.svg" alt="Markdwn" height="64">

**Éditeur et lecteur Markdown natif, multiplateforme.**

[![Licence](https://img.shields.io/badge/licence-MIT-09090B?style=flat-square)](LICENSE)
[![Plateformes](https://img.shields.io/badge/plateformes-Windows%20·%20macOS%20·%20Linux-5C5BF9?style=flat-square)](#installation)
[![Stack](https://img.shields.io/badge/stack-Tauri%20·%20Svelte%20·%20Rust-09090B?style=flat-square)](#stack-technique)
[![Statut](https://img.shields.io/badge/statut-en%20d%C3%A9veloppement-F5781D?style=flat-square)](#statut-du-projet)

[English version](README.en.md)

</div>

---

Markdwn traite la lecture et l'écriture comme deux usages distincts, avec une
interface qui s'adapte à chacun plutôt qu'un mode unique à tout faire.

## Fonctionnalités clés

- **Trois modes dédiés** — Lecture (`Ctrl+1`), Split (`Ctrl+2`), Zen (`Ctrl+3`) — chacun avec sa propre mise en page, pas un simple bouton de barre d'outils.
- **Rendu Markdown en Rust**, avec sommaire cliquable et scroll synchronisé entre l'éditeur et l'aperçu.
- **Suivi des liens relatifs** entre documents, y compris ancres et fichiers sans extension.
- **Éditeur CodeMirror 6** avec coloration syntaxique Markdown et formatage rapide (gras, italique, lien, liste, citation).
- **Onglets multi-documents**, thèmes clair et sombre, arborescence de dossier masquable.

## Raccourcis essentiels

| Raccourci | Action |
| --- | --- |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Lecture / Split / Zen |
| `Ctrl+O` / `Ctrl+S` / `Ctrl+N` | Ouvrir / Enregistrer / Nouveau |
| `Ctrl+B` | Afficher / masquer la barre de dossiers |
| `Ctrl+W` | Fermer l'onglet |
| `Ctrl+,` | Paramètres |

Liste complète dans le menu applicatif (clic sur le logo, en haut à gauche).

## Installation

Trois livrables sont produits pour Windows dans les [releases](../../releases) :

| Artefact | Usage |
| --- | --- |
| `Markdwn.exe` | Exécutable autonome, sans installation |
| `Markdwn_x64_en-US.msi` | Installeur MSI (déploiement d'entreprise) |
| `Markdwn_x64-setup.exe` | Installeur classique |

macOS et Linux sont sur la feuille de route.

> Les binaires ne sont pas signés : Windows SmartScreen affichera un
> avertissement « Éditeur inconnu » au premier lancement.

## Stack technique

**Tauri 2** (shell natif) · **Svelte 5** (interface) · **CodeMirror 6** (éditeur) · **Rust** (lecture disque et parsing Markdown).

## Statut du projet

En développement actif. Fonctionnel dès aujourd'hui : les trois modes, l'édition
et l'aperçu, le suivi des liens, les onglets, les thèmes, la configuration
persistée. À venir : images relatives dans l'aperçu, palette de commandes,
recherche multi-fichiers, coloration syntaxique des blocs de code, portage
macOS/Linux.

## Licence

[MIT](LICENSE)
