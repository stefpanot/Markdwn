#!/usr/bin/env python3
"""Génère le latest.json consommé par tauri-plugin-updater (endpoint statique).

Appelé par release.yml après publication de la release : chaque plateforme
pointe vers son installeur signé, la signature étant le contenu du fichier
`.sig` produit au build (minisign, clé publique dans tauri.conf.json).

Usage : updater-manifest.py <tag> <assets-dir>
"""

import glob
import json
import os
import subprocess
import sys

# Motif d'asset par plateforme (triplets tauri-plugin-updater). Seuls les
# formats que le plugin sait remplacer sont listés : NSIS sous Windows,
# AppImage sous Linux, .app.tar.gz sous macOS.
MAPPING = {
    "windows-x86_64": "Markdwn_*_x64-setup.exe",
    "linux-x86_64": "Markdwn_*_amd64.AppImage",
    "darwin-x86_64": "Markdwn_*_x64.app.tar.gz",
    "darwin-aarch64": "Markdwn_*_aarch64.app.tar.gz",
}


def fail(msg):
    print(f"::error::{msg}")
    sys.exit(1)


def main():
    if len(sys.argv) != 3:
        fail("usage : updater-manifest.py <tag> <assets-dir>")
    tag, assets_dir = sys.argv[1], sys.argv[2]
    repo = os.environ["GH_REPO"]

    def gh(*args):
        return subprocess.run(
            ["gh", "release", "view", tag, *args],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()

    notes = gh("--json", "body", "--jq", ".body")
    pub_date = gh("--json", "createdAt", "--jq", ".createdAt")

    platforms = {}
    for target, pattern in MAPPING.items():
        hits = sorted(glob.glob(os.path.join(assets_dir, pattern)))
        if not hits:
            print(f"::warning::pas d'installeur pour {target} ({pattern})")
            continue
        asset = hits[0]
        sig_path = asset + ".sig"
        if not os.path.exists(sig_path):
            fail(f"signature absente : {sig_path}")
        with open(sig_path, encoding="utf-8") as f:
            signature = f.read().strip()
        name = os.path.basename(asset)
        platforms[target] = {
            "signature": signature,
            "url": f"https://github.com/{repo}/releases/download/{tag}/{name}",
        }

    if not platforms:
        fail("aucune plateforme signée, latest.json vide")

    doc = {
        "version": tag,
        "notes": notes,
        "pub_date": pub_date,
        "platforms": platforms,
    }
    with open("latest.json", "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
    print(f"latest.json : {sorted(platforms)}")


if __name__ == "__main__":
    main()
