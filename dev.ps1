<#
    Lance Markdwn en développement, le construit, ou gère sa version.

    Pourquoi ce script : Visual Studio 2026 n'écrit plus la clé de registre
    HKLM\SOFTWARE\Microsoft\VisualStudio\SxS\VS7, sur laquelle repose une partie
    de l'auto-détection MSVC de rustc. Résultat : `link.exe` est bien installé
    mais introuvable par cargo. On charge donc l'environnement MSVC
    explicitement avant de lancer Tauri.

    Usage :  .\dev.ps1                  # tauri dev
             .\dev.ps1 build            # tauri build (vérifie la version d'abord)
             .\dev.ps1 test             # tests Rust
             .\dev.ps1 version          # affiche la version et vérifie la source unique
             .\dev.ps1 version 0.2.0    # monte la version
#>
param(
    [ValidateSet('dev', 'build', 'test', 'version')][string]$Task = 'dev',
    [string]$Value
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

$cargoToml = Join-Path $PSScriptRoot 'src-tauri\Cargo.toml'

function Get-AppVersion {
    $line = Select-String -Path $cargoToml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $line) { throw "Version introuvable dans $cargoToml" }
    $line.Matches[0].Groups[1].Value
}

<#
    La version de l'app vit UNIQUEMENT dans src-tauri\Cargo.toml.
    Tauri retombe dessus quand tauri.conf.json n'a pas de champ `version`, et
    package.json est un paquet privé qui n'en a pas besoin. Ce garde empêche un
    duplicata de réapparaître en silence puis de dériver.
#>
function Assert-SingleVersionSource {
    & node scripts/version.mjs check
    if ($LASTEXITCODE -ne 0) { throw 'Vérification de version échouée.' }
}

function Set-AppVersion([string]$next) {
    & node scripts/version.mjs set $next
    if ($LASTEXITCODE -ne 0) { throw 'Changement de version échoué.' }
}

<#
    Place le dossier du node épinglé par mise en tête du PATH.

    `mise exec -- npm …` est volontairement évité : le `--` ne traverse pas
    toujours l'appel natif selon les profils PowerShell (clap se plaint alors
    d'un <COMMAND> manquant), et `mise` peut être absent du PATH ou intercepté
    par un alias de profil. On résout donc le binaire explicitement, on demande
    le chemin de node (`mise which`, arguments positionnels simples), puis on
    appelle npm directement.
#>
function Enable-NodeEnv {
    $mise = @(
        "$env:LOCALAPPDATA\Microsoft\WinGet\Links\mise.exe",
        "$env:USERPROFILE\.local\bin\mise.exe",
        (Get-Command mise.exe -ErrorAction SilentlyContinue).Source
    ) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1

    $node = $null
    if ($mise) {
        $node = (& $mise which node 2>$null | Select-Object -First 1)
    }
    if (-not $node -or -not (Test-Path $node)) {
        # Dernier recours : un node quelconque déjà sur le PATH.
        $node = (Get-Command node -ErrorAction SilentlyContinue).Source
    }
    if (-not $node -or -not (Test-Path $node)) {
        throw "node introuvable. Lancer 'mise install' dans le projet ou installer Node."
    }
    $env:Path = "$(Split-Path $node);$env:Path"
    return $node
}

<#
    Exporte les variables de signature Tauri pour `tauri build`.

    Depuis l'ajout du plugin updater, `createUpdaterArtifacts` exige une
    signature : sans ces variables, le build local échoue à la fin. La clé
    privée reste hors du dépôt (~/.tauri/markdwn.key) ; le mot de passe est
    lu depuis ~/.tauri/markdwn.pass (à créer une fois, hors git). Une pièce
    manquante n'empêche que la production d'artefacts de mise à jour, donc
    on avertit plutôt qu'on n'échoue.
#>
function Enable-SigningEnv {
    $tauriDir = Join-Path $env:USERPROFILE '.tauri'
    $keyPath = Join-Path $tauriDir 'markdwn.key'
    $passPath = Join-Path $tauriDir 'markdwn.pass'
    if (Test-Path $keyPath) {
        # Contenu ET chemin : le bundler de mise à jour ne lit que la variable
        # de contenu, le sous-commande `signer` accepte les deux. On pose
        # donc les deux pour couvrir les deux code paths.
        $env:TAURI_SIGNING_PRIVATE_KEY = [IO.File]::ReadAllText($keyPath).Trim()
        $env:TAURI_SIGNING_PRIVATE_KEY_PATH = $keyPath
        if (Test-Path $passPath) {
            $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = (Get-Content $passPath -Raw).Trim()
        } else {
            Write-Host "Mot de passe de signature absent ($passPath) : creation d'un .pass necessaire pour signer." -ForegroundColor Yellow
        }
    } else {
        Write-Host "Cle de signature absente ($keyPath) : pas d'artefacts de mise a jour signes." -ForegroundColor Yellow
    }
}

# --- tâche version : pas besoin de l'environnement MSVC ---
if ($Task -eq 'version') {
    Enable-NodeEnv | Out-Null
    if ($Value) { Set-AppVersion $Value }
    Assert-SingleVersionSource
    Write-Host ("Version de l'application : {0}" -f (Get-AppVersion))
    Write-Host "Source unique : src-tauri\Cargo.toml (verifie)" -ForegroundColor DarkGray
    exit 0
}

# --- environnement MSVC ---
$vsShell = Get-ChildItem `
    'C:\Program Files\Microsoft Visual Studio\*\*\Common7\Tools\Launch-VsDevShell.ps1',
    'C:\Program Files (x86)\Microsoft Visual Studio\*\*\Common7\Tools\Launch-VsDevShell.ps1' `
    -ErrorAction SilentlyContinue | Select-Object -First 1

if (-not $vsShell) {
    throw "Launch-VsDevShell.ps1 introuvable. La charge de travail « Développement Desktop en C++ » de Visual Studio est-elle installée ?"
}

# Launch-VsDevShell.ps1 résout vswhere via %ProgramFiles(x86)%, une variable
# MACHINE que certaines machines perdent — le chemin se résout alors en
# « \Microsoft Visual Studio\Installer\vswhere.exe » et tout s'écroule. On
# localise vswhere soi-même et on le passe explicitement (-VsWherePath).
$vsWhere = Get-ChildItem `
    'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe',
    'C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe' `
    -ErrorAction SilentlyContinue | Select-Object -First 1

if (-not $vsWhere) {
    throw "vswhere.exe introuvable. Visual Studio est-il installé ?"
}

& $vsShell.FullName -Arch amd64 -HostArch amd64 -SkipAutomaticLocation -VsWherePath $vsWhere.FullName | Out-Null
if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    throw "link.exe reste introuvable après le chargement de l'environnement MSVC."
}

# --- cargo ---
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) { $env:Path = "$cargoBin;$env:Path" }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo introuvable. Installer Rust : winget install --id Rustlang.Rustup"
}

Write-Host "MSVC + Rust prets." -ForegroundColor DarkGray
Write-Host ("Node : {0}" -f (Enable-NodeEnv)) -ForegroundColor DarkGray

switch ($Task) {
    'dev' { & npm run tauri dev }
    'build' {
        # Ne jamais livrer un binaire dont la version a dérivé.
        Assert-SingleVersionSource
        Enable-SigningEnv
        Write-Host ("Build de la version {0}" -f (Get-AppVersion)) -ForegroundColor DarkGray
        & npm run tauri build
    }
    'test' { & cargo test --manifest-path src-tauri/Cargo.toml }
}
exit $LASTEXITCODE
