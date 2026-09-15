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

# --- tâche version : pas besoin de l'environnement MSVC ---
if ($Task -eq 'version') {
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

& $vsShell.FullName -Arch amd64 -HostArch amd64 -SkipAutomaticLocation | Out-Null
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

switch ($Task) {
    'dev' { & mise exec -- npm run tauri dev }
    'build' {
        # Ne jamais livrer un binaire dont la version a dérivé.
        Assert-SingleVersionSource
        Write-Host ("Build de la version {0}" -f (Get-AppVersion)) -ForegroundColor DarkGray
        & mise exec -- npm run tauri build
    }
    'test' { & cargo test --manifest-path src-tauri/Cargo.toml }
}
exit $LASTEXITCODE
