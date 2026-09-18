# Publier une version

La version a une **source unique** : `src-tauri/Cargo.toml` (gardée par
`scripts/version.mjs`, testée par `npm run test:version` et par la CI).

## Voie normale : le workflow « Prepare release » (un clic)

1. Merger la PR du travail à publier sur `main`.
2. Sur GitHub : **Actions → Prepare release → Run workflow**, saisir la version
   (`X.Y.Z`, ou `X.Y.Z-beta.N` pour une beta).
3. Le workflow :
   - écrit la version dans `Cargo.toml` et `Cargo.lock`, commit
     `chore: release vX.Y.Z` sur `main`, tag annoté `vX.Y.Z`, push ;
   - appelle `release.yml` qui vérifie tag ↔ version, lance les tests, build
     les installeurs des 4 cibles (Windows NSIS + MSI, Linux deb/rpm/AppImage,
     macOS arm64 + Intel) et publie la release GitHub avec `SHA256SUMS.txt`.
4. Une beta (`-beta.N`) est marquée *pre-release*, non « latest », et ne
   produit pas de MSI (versions numériques seulement).

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
