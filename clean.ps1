<#
    Récupère l'espace disque pris par les artefacts de compilation.

    Trois niveaux, du moins au plus radical :

      .\clean.ps1                 Sûr. Cache incrémental, notre crate, cache
                                  SvelteKit. Les dépendances compilées restent,
                                  donc le prochain build est rapide.

      .\clean.ps1 -Release        Ci-dessus, plus TOUT l'arbre release — après
                                  avoir recopié les installeurs dans dist\.
                                  C'est le meilleur rapport gain/inconvénient :
                                  l'arbre release ne sert qu'à produire un
                                  livrable, et le livrable est conservé.

      .\clean.ps1 -All            Tout l'arbre target. Prochain build complet
                                  (~3 min). Ajouter -IncludeNodeModules pour
                                  aussi vider node_modules (npm install requis
                                  ensuite, donc du réseau).

    -DryRun affiche ce qui serait libéré sans rien supprimer.
#>
[CmdletBinding()]
param(
    [switch]$Release,
    [switch]$All,
    [switch]$IncludeNodeModules,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

$target = Join-Path $PSScriptRoot 'src-tauri\target'

function Get-SizeMb($path) {
    if (-not (Test-Path $path)) { return 0 }
    $sum = (Get-ChildItem $path -Recurse -File -Force -ErrorAction SilentlyContinue |
        Measure-Object -Sum Length).Sum
    [math]::Round(($sum / 1MB), 0)
}

# --- ne refuser QUE sur ce qui verrouille vraiment ce qu'on va supprimer ---
# Un `node` quelconque de la machine ne concerne pas ce projet, et l'app en
# cours vit dans target\debug : elle n'empêche pas de nettoyer l'arbre release.
function Get-Blockers {
    $blockers = @()

    foreach ($n in 'cargo', 'rustc') {
        if (Get-Process $n -ErrorAction SilentlyContinue) { $blockers += "$n (build en cours)" }
    }

    if ($All -and (Get-Process 'markdownedit' -ErrorAction SilentlyContinue)) {
        $blockers += 'markdownedit (son binaire est dans target\debug)'
    }

    $here = $PSScriptRoot
    $ourNode = Get-CimInstance Win32_Process -Filter "Name='node.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -and $_.CommandLine -like "*$here*" }
    if ($ourNode) { $blockers += 'serveur de dev Vite de ce projet' }

    $blockers
}

$blockers = if ($DryRun) { @() } else { Get-Blockers }
if ($blockers) {
    Write-Host "Nettoyage impossible, ces processus verrouillent les fichiers visés :" -ForegroundColor Yellow
    $blockers | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }
    exit 1
}

$before = Get-SizeMb $PSScriptRoot
Write-Host ("Avant : {0:N0} Mo" -f $before) -ForegroundColor DarkGray

$plan = [System.Collections.Generic.List[object]]::new()
function Plan($label, $path) {
    $mb = Get-SizeMb $path
    if ($mb -gt 0 -or (Test-Path $path)) { $plan.Add([pscustomobject]@{ Label = $label; Path = $path; Mo = $mb }) }
}

# Les entrées ne doivent jamais s'emboîter, sinon le total compte deux fois :
# `target` contient déjà `release` et `debug\incremental`.
$keepBundle = $Release -or $All

Plan 'cache SvelteKit' (Join-Path $PSScriptRoot '.svelte-kit')

if ($All) {
    Plan 'arbre target complet' $target
    Plan 'sortie front' (Join-Path $PSScriptRoot 'build')
    if ($IncludeNodeModules) {
        Plan 'node_modules' (Join-Path $PSScriptRoot 'node_modules')
    }
} else {
    Plan 'cache incrémental (debug)' (Join-Path $target 'debug\incremental')
    if ($Release) {
        Plan 'arbre release' (Join-Path $target 'release')
    }
}

$plan | Select-Object Label, Mo | Format-Table -AutoSize

if ($DryRun) {
    Write-Host ("Simulation : {0:N0} Mo seraient liberes." -f ($plan | Measure-Object -Sum Mo).Sum) -ForegroundColor Cyan
    exit 0
}

# --- préserver les livrables avant de jeter l'arbre release ---
if ($keepBundle) {
    $bundle = Join-Path $target 'release\bundle'
    $exe = Join-Path $target 'release\MarkdownEdit.exe'
    if ((Test-Path $bundle) -or (Test-Path $exe)) {
        $dist = Join-Path $PSScriptRoot 'dist'
        New-Item -ItemType Directory -Force -Path $dist | Out-Null
        if (Test-Path $bundle) {
            Get-ChildItem $bundle -Recurse -File -Include *.msi, *.exe -ErrorAction SilentlyContinue |
                ForEach-Object { Copy-Item $_.FullName $dist -Force }
        }
        if (Test-Path $exe) { Copy-Item $exe $dist -Force }
        $kept = (Get-ChildItem $dist -File -ErrorAction SilentlyContinue).Count
        Write-Host "Livrables recopies dans dist\ : $kept fichier(s)" -ForegroundColor DarkGray
    }
}

# --- notre crate seule : les dépendances compilées restent ---
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if ((Test-Path $cargo) -and -not $All) {
    foreach ($args in @(@('clean', '-p', 'markdownedit'), @('clean', '-p', 'markdownedit', '--release'))) {
        & $cargo @args --manifest-path 'src-tauri\Cargo.toml' 2>&1 | Out-Null
    }
}

foreach ($item in $plan) {
    if (Test-Path $item.Path) {
        Remove-Item $item.Path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

$after = Get-SizeMb $PSScriptRoot
Write-Host ""
Write-Host ("Apres : {0:N0} Mo  —  {1:N0} Mo liberes" -f $after, ($before - $after)) -ForegroundColor Green
if ($All) {
    Write-Host "Le prochain build sera complet (~3 min)." -ForegroundColor DarkGray
    if ($IncludeNodeModules) { Write-Host "Lance 'mise exec -- npm install' avant." -ForegroundColor DarkGray }
}
