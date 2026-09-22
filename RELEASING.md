# Publier une version

La version a une **source unique** : `src-tauri/Cargo.toml` (gardée par
`scripts/version.mjs`, testée par `npm run test:version` et par la CI).

## Prérequis : les secrets de signature (une seule fois)

L'auto-update exige des installeurs signés (minisign). Deux secrets doivent
exister sur le dépôt (`Settings → Secrets and variables → Actions`) :

- `TAURI_SIGNING_PRIVATE_KEY` — contenu de la clé privée
  (`npm run tauri -- signer generate --ci --password … --write-keys …`) ;
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — son mot de passe.

La clé publique correspondante vit dans `tauri.conf.json` (non secrète). Sans
ces secrets, le build de release échoue volontairement : mieux vaut un échec
explicite que des installeurs non signés publiés en silence. Garder le fichier
`.key` en lieu sûr : sa perte rend les futures mises à jour impossibles à
signer.

## Voie normale : le workflow « Prepare release » (un clic)

1. Merger la PR du travail à publier sur `main`.
2. Sur GitHub : **Actions → Prepare release → Run workflow**, saisir la version
   (`X.Y.Z`, ou `X.Y.Z-beta.N` pour une beta). Le workflow refuse de tourner
   depuis une autre branche que `main` : son push cible `main` directement.
3. Le workflow :
   - écrit la version dans `Cargo.toml` et `Cargo.lock`, commit
     `chore: release vX.Y.Z` sur `main`, tag annoté `vX.Y.Z`, push ;
   - appelle `release.yml` qui vérifie tag ↔ version, lance les tests, build
     les installeurs des 4 cibles (Windows NSIS + MSI, Linux deb/rpm/AppImage,
     macOS arm64 + Intel), les signe, et publie la release GitHub avec
     `SHA256SUMS.txt` et `latest.json` (point d'entrée de l'auto-update).
4. C'est le **format de la version** qui décide du canal, pas la branche :
   `0.2.0` → release stable ; `0.2.0-beta.1` → pre-release, non « latest »,
   sans MSI (versions numériques seulement).

## Publier une beta : le cycle complet

Même bouton, même workflow — seule la version saisie change. Exemple de cycle :

1. **Première beta** : Actions → Prepare release → `0.2.0-beta.1`.
   Résultat : release GitHub estampillée *Pre-release*, la release stable
   précédente reste celle mise en avant (« Latest ») sur la page du dépôt.
2. **Beta suivante** : Prepare release → `0.2.0-beta.2`, et ainsi de suite.
   C'est toi qui incrémentes le `N` (obligatoire, à partir de 1) : le script
   ne devine pas le numéro précédent.
3. **Stable finale** : Prepare release → `0.2.0`. Le workflow fait passer
   `Cargo.toml` de `0.2.0-beta.2` à `0.2.0` comme n'importe quel bump.

Entre une beta et la stable, `Cargo.toml` sur `main` porte donc la version
beta : c'est normal et sans conséquence — la CI quotidienne ne publie rien,
seul le tag déclenche une release. Sous Windows, une beta ne produit pas de
MSI (les versions produit MSI sont purement numériques) : NSIS + portable
seulement.

Pourquoi l'appel direct entre workflows : un tag poussé avec `GITHUB_TOKEN` ne
déclenche pas les workflows `on: push` (limitation GitHub). `release.yml` est
donc aussi un `workflow_call` que « Prepare release » invoque après le tag.

## Voie manuelle (identique, sans le bump automatisé)

```powershell
.\dev.ps1 version 0.2.0      # écrit Cargo.toml + Cargo.lock, vérifie
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: release v0.2.0"
git tag -a v0.2.0 -m "Markdwn v0.2.0"
git push origin main v0.2.0  # le push du tag déclenche release.yml
```

## Builder les installeurs sans publier

**Actions → Release → Run workflow** : mêmes builds, artefacts téléchargeables
14 jours, aucune release créée.

## Points de vigilance

- Le tag doit correspondre exactement à la version de `Cargo.toml`
  (`v` + version), sinon le workflow échoue avant de builder.
- Le commit de release part de `main` : si une protection de branche bloque le
  push du bot, le faire en voie manuelle ou ajuster la protection.
- macOS : signature ad-hoc, sans notarisation — Gatekeeper affichera un
  avertissement, comme SmartScreen sous Windows.
- Auto-update : l'endpoint (`releases/latest/download/latest.json`) ne
  résout jamais les *Pre-release*. Tant qu'aucune release stable n'existe,
  les beta ne se mettent pas à jour toutes seules — c'est voulu, le canal
  stable est le seul servi. NSIS se met à jour ; le MSI, lui, ne le peut pas
  (le plugin l'ignore).
