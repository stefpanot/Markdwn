# K3 — Aperçu Markdown dans le volet Aperçu de l'Explorateur Windows

Cadrage proposé le 2026-10-09, après exploration de faisabilité. Ce document
est le point d'entrée de la session d'implémentation : périmètre, décisions,
points d'ancrage. Ne pas re-débattre ce qui est marqué **décidé** ; le reste
est à l'appréciation de l'implémentation.

Principes du projet (rappel, inchangés) : tout ce qui touche au disque et au
parsing vit en Rust ; la mise en page reste dans la webview ; commentaires en
français expliquant le *pourquoi*.

## Intention

Dans l'Explorateur Windows 11, sélectionner un `.md` avec le volet « Aperçu »
ouvert n'affiche rien (ou le texte brut). Windows sait le faire pour les
images via des *Preview Handlers* — des DLL COM chargées dans `explorer.exe`
qui peignent dans un HWND fourni par le système. Markdwn fournira ce handler
pour ses extensions : l'aperçu Explorer sera rendu par le même moteur que
l'aperçu de l'app.

Alternative zéro code à connaître : PowerToys propose déjà un aperçu Markdown
dans ses « File Explorer add-ons ». C'est leur rendu, pas le nôtre — K3 vaut
comme différenciation produit, pas comme déblocage fonctionnel.

## Périmètre (les 8 items)

### 1. Crate dédiée `preview-handler` (cdylib)

- Nouvelle crate dans le workspace, à côté de `src-tauri`, produisant
  `preview-handler.dll`.
- Implémente les interfaces COM requises : `IPreviewHandler`,
  `IInitializeWithStream` (le contenu arrive en mémoire via `IStream`,
  jamais par chemin — l'Explorateur fonctionne aussi pour les fichiers non
  locaux), `IOleWindow`, `IObjectWithSite`. `IPreviewHandlerVisuals` si le
  thème l'exige.
- La crate `windows` (COM) et éventuellement `webview2-com` (embedding
  WebView2 hors Tauri) — choix exact laissé à l'implémentation.
- Exigences de robustesse : la DLL vit dans `explorer.exe`. Aucune panic à
  travers la FFI, aucun `unwrap` dans le code appelé par COM, threads gérés
  explicitement (le handler COM est invoqué sur un thread de fond ;
  WebView2 veut son propre thread à message loop). Le profile release
  (`panic = "abort"`) est déjà en place — le conserver.

### 2. Rendu partagé avec l'app

- **Décidé** : l'aperçu Explorer est rendu par le même pipeline que l'app
  (pulldown-cmark + syntect + ammonia), donc par le code existant
  `src-tauri/src/markdown.rs`.
- `markdwn_lib` déclare `crate-type = ["staticlib", "cdylib", "rlib"]` : la
  nouvelle DLL dépend de la **rlib** et réutilise `markdown::render` tel quel.
  Aucune duplication du moteur, aucune dérive de rendu entre app et aperçu.

### 3. Affichage : WebView2 embarqué

- **Décidé** : le HTML rendu s'affiche dans une WebView2 embarquée sur le
  HWND fourni par l'Explorateur. Le runtime WebView2 est déjà garanti
  présent (l'app en dépend), donc aucun prérequis supplémentaire.
- Feuille de style : extrait minimal de `Preview.svelte` (typographie de
  lecture, code, tableaux) compilé en CSS texte dans la DLL — l'aperçu
  ressemble au mode Lecture sans copier toute la webview. Pas de JS.
- Performance : le démarrage WebView2 coûte quelques centaines de ms à
  l'ouverture du volet. Mesurer ; si insuffisant, envisager un rendu
  statique (DirectWrite) en fallback — hors périmètre par défaut.

### 4. Enregistrement COM par utilisateur (HKCU), sans admin

- **Décidé** : l'enregistrement est écrit par l'app dans
  `HKCU\Software\Classes` (clé shellex `{8895b1c6-b41f-4c1c-a562-0d564250836f}`
  par extension + CLSID de la DLL). Fusion HKCR = visible par l'Explorateur
  sans élévation, cohérent avec l'install mode « pour moi uniquement ».
- Extensions couvertes : `md`, `markdown`, `mdown`, `mkd` (les mêmes que
  l'association « ouvrir avec »).
- Écrit au premier lancement de l'app (pas par l'installeur) : l'installeur
  n'a qu'à copier la DLL, l'auto-update NSIS n'a rien de spécial à faire.

### 5. Réglage dans Paramètres

- Case « Aperçu Markdown dans l'Explorateur » (Espace de travail), **activé
  par défaut**.
- Persisté dans la config (champ optionnel, sans migration — même mécanisme
  que K2.8). Décocher = désinscrire les clés au prochain lancement.

### 6. Empaquetage

- La DLL rejoint le dossier d'installation via les ressources du bundle
  Tauri (à côté de `markdwn.exe`, chemin stable pour la CLSID).
- Rien à faire dans le NSIS au-delà de la copie (voir item 4). **À valider
  en conditions réelles** : le comportement de la désinstallation (clés
  résiduelles inoffensives mais à nettoyer si possible) et de l'auto-update
  (remplacement de la DLL sur place = enregistrement conservé, à vérifier).

### 7. Désinstallation

- Le NSIS supprime l'app : les clés HKCU résiduelles pointeraient vers une
  DLL absente. L'Explorateur gère ce cas (aperçu indisponible, pas de
  plantage), mais le propre est de les retirer : hook de désinstallation si
  le bundler l'expose, sinon documenter la limitation dans la ROADMAP.

### 8. Tests

- Rendu : déjà couvert par les tests existants de `markdown.rs` (réutilisé
  tel quel).
- Nouveaux tests : helper d'écriture/lecture du registre (clés attendues,
  extensions couvertes, désinscription), helper d'extraction du contenu
  depuis `IStream` (mockable si la logique est isolée hors COM).
- Le COM lui-même ne se teste pas en unitaire : validation manuelle sur
  Windows 11 (volet Aperçu, navigation clavier entre fichiers, fichiers
  lourds, .md avec images locales — attendu : images non résolues, rendu
  net quand même).

## Points d'ancrage dans le code

| Sujet | Fichier | Repère |
|---|---|---|
| Moteur de rendu | `src-tauri/src/markdown.rs` | `markdown::render` (réutilisé via rlib) |
| Crate existante | `src-tauri/Cargo.toml` | `crate-type` inclut déjà `rlib` |
| Commande d'enregistrement | `src-tauri/src/lib.rs` | nouvelle commande + appel au lancement |
| Registre Windows | crate `winreg` (nouvelle dépendance, Windows only) | — |
| Config | `src-tauri/src/config.rs` | champ optionnel, sans migration |
| Config front | `src/lib/state.svelte.ts` | `toConfig` / `applyConfig` / `resetSettings` |
| Case Paramètres | `src/lib/components/SettingsPanel.svelte` | section Espace de travail |
| Empaquetage | `src-tauri/tauri.conf.json` | `bundle.resources` pour la DLL |
| Layout lecture | `src/lib/components/Preview.svelte` | source d'inspiration du CSS statique |

## Hors périmètre (rappels)

- *Thumbnail Handler* Windows (aperçus dans les icônes) : autre interface,
  autre chantier.
- macOS (Quick Look) : l'équivalent existe — une extension Quick Look fournit
  du HTML via `QLPreviewingController`, ce qui collerait bien au rendu
  `markdown.rs`. Nuances : macOS affiche DÉJÀ les `.md` en texte brut dans
  Quick Look (le gain est un rendu mis en forme, pas une fonction absente),
  et le coût réel est l'empaquetage (cible Xcode, entitlements, notarisation)
  plutôt que le rendu. Chantier séparé, à réévaluer après K3.
- Linux : pas de volet d'aperçu unifié (Nautilus passe par Sushi, Dolphin a
  ses propres plugins) ; la seule spec transversale est le thumbnailer
  freedesktop, qui produit des vignettes d'icônes, pas un aperçu de lecture.
  Fragmenté, ROI faible : hors scope sauf demande explicite.
- Rendu différent de l'app (thème sombre dans le volet, etc.) : l'aperçu
  suit le CSS statique choisi en item 3, rien de configurable.
- Interaction dans l'aperçu (clics, scroll synchronisé) : le handler
  affiche, il ne navigue pas.

## Definition of done

- Un `.md` sélectionné dans l'Explorateur Windows 11 s'affiche dans le
  volet Aperçu, rendu par le moteur Markdwn, sans droits admin, sur
  install NSIS propre ET après auto-update.
- La case Paramètres active/désactive l'aperçu (après relance ou immédiat,
  à l'appréciation de l'implémentation).
- `cargo test` et `npm run check` proprets ; tests pour tout helper Rust
  nouveau testable.
- ROADMAP.md mis à jour (entrée « Livré récemment » ou nouvelle piste
  selon l'état d'avancement).
- Pas de bump de version : `dev.ps1 version` reste la seule porte.

## Suggested session order

1. Crate COM minimale qui peint un HWND de test uni (valide la boucle
   threads/message loop).
2. `IInitializeWithStream` → extraction du texte → rendu `markdown.rs` →
   WebView2 embarquée.
3. Enregistrement HKCU + commande Tauri + branchement au lancement.
4. Config + Paramètres.
5. Empaquetage (resources), tests, validation installée/auto-update.
