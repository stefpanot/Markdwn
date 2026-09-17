<div align="center">

<img src="assets/logo-lockup.svg" alt="Markdwn" height="64">

**Native, cross-platform Markdown editor and reader.**

[![License](https://img.shields.io/badge/license-MIT-09090B?style=flat-square)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20·%20macOS%20·%20Linux-5C5BF9?style=flat-square)](#installation)
[![Stack](https://img.shields.io/badge/stack-Tauri%20·%20Svelte%20·%20Rust-09090B?style=flat-square)](#tech-stack)
[![Status](https://img.shields.io/badge/status-in%20development-F5781D?style=flat-square)](#project-status)

[Version française](README.md)

</div>

---

Markdwn treats reading and writing as two different jobs, with an interface
that adapts to each rather than one layout trying to serve both.

## Key features

- **Three dedicated modes** — Reading (`Ctrl+1`), Split (`Ctrl+2`), Zen (`Ctrl+3`) — each with its own layout, not a toolbar toggle.
- **Rust-powered Markdown rendering**, with a clickable outline and synced scrolling between editor and preview.
- **Relative link resolution** between documents, including anchors and extension-less links.
- **Relative images rendered in the preview** (`![…](images/x.png)` and raw `<img src="…">` alike), resolved from the document's folder.
- **CodeMirror 6 editor** with Markdown syntax highlighting and quick formatting (bold, italic, link, list, quote).
- **Multi-document tabs**, light and dark themes, a collapsible folder tree.
- **Command palette** (`Ctrl+K`) with fuzzy search over the open folder's files and every app command; `>` restricts it to commands.

## Essential shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Reading / Split / Zen |
| `Ctrl+O` / `Ctrl+S` / `Ctrl+N` | Open / Save / New |
| `Ctrl+K` | Command palette |
| `Ctrl+B` | Toggle folder sidebar |
| `Ctrl+W` | Close tab |
| `Ctrl+,` | Settings |

Full list in the app menu (click the logo, top left).

## Installation

Three artifacts are built for Windows, available in [releases](../../releases):

| Artifact | Use |
| --- | --- |
| `Markdwn.exe` | Standalone executable, no install |
| `Markdwn_x64_en-US.msi` | MSI installer (enterprise deployment) |
| `Markdwn_x64-setup.exe` | Classic installer |

macOS and Linux are on the roadmap.

> Binaries are unsigned: Windows SmartScreen will show an "Unknown publisher"
> warning on first launch.

## Tech stack

**Tauri 2** (native shell) · **Svelte 5** (UI) · **CodeMirror 6** (editor) · **Rust** (disk access and Markdown parsing).

> To develop: `.\dev.ps1` (Windows) loads the MSVC environment, then runs
> `tauri dev`. Running `npm run dev` alone opens the UI in a plain browser,
> **without disk access** — a degraded mode meant for UI work only, not a
> supported usage: Markdwn is a desktop application.

## Project status

Actively developed. Working today: all three modes, editing and preview, link
following, relative images in preview, tabs, themes, persisted configuration,
command palette. Coming up: multi-file search, code block syntax highlighting,
macOS/Linux builds.

## License

[MIT](LICENSE)
