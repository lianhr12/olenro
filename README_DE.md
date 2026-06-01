<div align="center">

# Olenro

> Based on [CC Switch](https://github.com/farion1231/cc-switch) — open source under the MIT License.

### Der All-in-One-Manager für Claude Code, Codex, Gemini CLI, OpenCode, OpenClaw & Hermes Agent

[![Version](https://img.shields.io/github/v/release/lianhr12/olenro?color=blue&label=version)](https://github.com/lianhr12/olenro/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/lianhr12/olenro/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)
[![Downloads](https://img.shields.io/github/downloads/lianhr12/olenro/total)](https://github.com/lianhr12/olenro/releases/latest)


### 🌐 Die einzige offizielle Website: **[olenro.com](https://olenro.com)**

[English](README.md) | [中文](README_ZH.md) | [日本語](README_JA.md) | Deutsch | [Changelog](CHANGELOG.md)

</div>

## Warum Olenro?

Modernes KI-gestütztes Programmieren stützt sich auf CLI-Werkzeuge wie Claude Code, Codex, Gemini CLI, OpenCode und OpenClaw — doch jedes hat sein eigenes Konfigurationsformat. Der Wechsel des API-Anbieters bedeutet, JSON-, TOML- oder `.env`-Dateien von Hand zu bearbeiten, und es gibt keine einheitliche Möglichkeit, MCP und Skills über mehrere Werkzeuge hinweg zu verwalten.

**Olenro** gibt Ihnen eine einzige Desktop-App, um alle fünf CLI-Werkzeuge zu verwalten. Statt Konfigurationsdateien von Hand zu bearbeiten, erhalten Sie eine visuelle Oberfläche, um Anbieter mit einem Klick zu importieren und sofort zwischen ihnen zu wechseln — mit 50+ integrierten Anbieter-Presets, einheitlicher MCP- und Skills-Verwaltung und schnellem Umschalten über das System-Tray. Das Ganze gestützt auf eine zuverlässige SQLite-Datenbank mit atomaren Schreibvorgängen, die Ihre Konfigurationen vor Beschädigung schützen.

- **Eine App, fünf CLI-Werkzeuge** — Verwalten Sie Claude Code, Codex, Gemini CLI, OpenCode und OpenClaw über eine einzige Oberfläche
- **Kein manuelles Bearbeiten mehr** — 50+ Anbieter-Presets einschließlich AWS Bedrock, NVIDIA NIM und Community-Relays; einfach auswählen und umschalten
- **Einheitliche MCP- & Skills-Verwaltung** — Ein Panel zur Verwaltung von MCP-Servern und Skills über vier Apps hinweg mit bidirektionaler Synchronisierung
- **Schnellumschaltung über System-Tray** — Wechseln Sie Anbieter sofort über das Tray-Menü, ohne die vollständige App öffnen zu müssen
- **Cloud-Synchronisierung** — Synchronisieren Sie Anbieterdaten geräteübergreifend über Dropbox, OneDrive, iCloud oder WebDAV-Server
- **Plattformübergreifend** — Native Desktop-App für Windows, macOS und Linux, gebaut mit Tauri 2
- **Integrierte Hilfsprogramme** — Enthält diverse Hilfsprogramme für die Login-Bestätigung beim Erststart, das Umgehen von Signaturen, die Synchronisierung von Plugin-Erweiterungen und mehr

## Screenshots

|                  Hauptoberfläche                   |                  Anbieter hinzufügen                  |
| :-----------------------------------------------: | :--------------------------------------------: |
| ![Hauptoberfläche](assets/screenshots/main-en.png) | ![Anbieter hinzufügen](assets/screenshots/add-en.png) |

## Funktionen

[Vollständiges Changelog](CHANGELOG.md) | [Release Notes](docs/release-notes/v1.0.0-en.md)

### Anbieterverwaltung

- **5 CLI-Werkzeuge, 50+ Presets** — Claude Code, Codex, Gemini CLI, OpenCode, OpenClaw; Schlüssel kopieren und mit einem Klick importieren
- **Universelle Anbieter** — Eine Konfiguration synchronisiert sich mit mehreren Apps (OpenCode, OpenClaw)
- Umschaltung mit einem Klick, Schnellzugriff über System-Tray, Sortierung per Drag-and-drop, Import/Export

### Proxy & Failover

- **Lokaler Proxy mit Hot-Switching** — Formatkonvertierung, automatisches Failover, Circuit Breaker, Anbieter-Health-Monitoring und Request-Rectifier
- **Übernahme auf App-Ebene** — Claude, Codex oder Gemini unabhängig über den Proxy leiten, bis hinunter auf einzelne Anbieter

### MCP, Prompts & Skills

- **Einheitliches MCP-Panel** — Verwalten Sie MCP-Server über 4 Apps hinweg mit bidirektionaler Synchronisierung und Deep-Link-Import
- **Prompts** — Markdown-Editor mit App-übergreifender Synchronisierung (CLAUDE.md / AGENTS.md / GEMINI.md) und Backfill-Schutz
- **Skills** — Installation mit einem Klick aus GitHub-Repositorys oder ZIP-Dateien, Verwaltung eigener Repositorys, mit Unterstützung für Symlinks und Dateikopien

### Nutzungs- & Kostenverfolgung

- **Nutzungs-Dashboard** — Verfolgen Sie Ausgaben, Anfragen und Token mit Trenddiagrammen, detaillierten Anfrageprotokollen und eigener Preisgestaltung pro Modell

### Session Manager & Workspace

- Durchsuchen, suchen und stellen Sie den Gesprächsverlauf über alle Apps hinweg wieder her
- **Workspace-Editor** (OpenClaw) — Bearbeiten Sie Agent-Dateien (AGENTS.md, SOUL.md usw.) mit Markdown-Vorschau

### System & Plattform

- **Cloud-Synchronisierung** — Eigenes Konfigurationsverzeichnis (Dropbox, OneDrive, iCloud, NAS) und WebDAV-Server-Synchronisierung
- **Deep Link** (`olenro://`) — Importieren Sie Anbieter, MCP-Server, Prompts und Skills per URL
- Dunkles / Helles / System-Theme, automatischer Start, automatischer Updater, atomare Schreibvorgänge, automatische Backups, i18n (zh/en/ja)

## FAQ

<details>
<summary><strong>Welche KI-CLI-Werkzeuge unterstützt Olenro?</strong></summary>

Olenro unterstützt fünf Werkzeuge: **Claude Code**, **Codex**, **Gemini CLI**, **OpenCode** und **OpenClaw**. Jedes Werkzeug verfügt über dedizierte Anbieter-Presets und Konfigurationsverwaltung.

</details>

<details>
<summary><strong>Muss ich das Terminal nach einem Anbieterwechsel neu starten?</strong></summary>

Bei den meisten Werkzeugen ja — starten Sie Ihr Terminal oder das CLI-Werkzeug neu, damit die Änderungen wirksam werden. Die Ausnahme ist **Claude Code**, das derzeit das Hot-Switching von Anbieterdaten ohne Neustart unterstützt.

</details>

<details>
<summary><strong>Meine Plugin-Konfiguration ist nach einem Anbieterwechsel verschwunden — was ist passiert?</strong></summary>

Olenro bietet eine Funktion „Gemeinsames Konfigurations-Snippet", um gemeinsame Daten (über API-Schlüssel und Endpunkte hinaus) zwischen Anbietern weiterzugeben. Gehen Sie zu „Anbieter bearbeiten" → „Panel für gemeinsame Konfiguration" → klicken Sie auf „Aus aktuellem Anbieter extrahieren", um alle gemeinsamen Daten zu speichern. Aktivieren Sie beim Anlegen eines neuen Anbieters die Option „Gemeinsame Konfiguration schreiben" (standardmäßig aktiviert), um die Plugin-Daten in den neuen Anbieter aufzunehmen. Alle Ihre Konfigurationspunkte bleiben im Standardanbieter erhalten, der beim ersten Start der App importiert wurde.

</details>

<details>
<summary><strong>Installation unter macOS</strong></summary>

Olenro für macOS ist von Apple code-signiert und notarisiert. Sie können es direkt herunterladen und installieren — es sind keine zusätzlichen Schritte erforderlich. Wir empfehlen die Verwendung des `.dmg`-Installationsprogramms.

</details>

<details>
<summary><strong>Warum kann ich den aktuell aktiven Anbieter nicht löschen?</strong></summary>

Olenro folgt dem Designprinzip der „minimalen Eingriffstiefe" — selbst wenn Sie die App deinstallieren, funktionieren Ihre CLI-Werkzeuge weiterhin normal. Das System behält immer eine aktive Konfiguration bei, da das Löschen aller Konfigurationen das entsprechende CLI-Werkzeug unbrauchbar machen würde. Wenn Sie ein bestimmtes CLI-Werkzeug selten verwenden, können Sie es in den Einstellungen ausblenden. Wie Sie zurück zum offiziellen Login wechseln, erfahren Sie in der nächsten Frage.

</details>

<details>
<summary><strong>Wie wechsle ich zurück zum offiziellen Login?</strong></summary>

Fügen Sie einen offiziellen Anbieter aus der Preset-Liste hinzu. Führen Sie nach dem Wechsel den Abmelde-/Anmelde-Vorgang aus; anschließend können Sie frei zwischen dem offiziellen Anbieter und Drittanbietern wechseln. Codex unterstützt den Wechsel zwischen verschiedenen offiziellen Anbietern, was das Umschalten zwischen mehreren Plus- oder Team-Konten erleichtert.

</details>

<details>
<summary><strong>Wo werden meine Daten gespeichert?</strong></summary>

- **Datenbank**: `~/.olenro/olenro.db` (SQLite — Anbieter, MCP, Prompts, Skills)
- **Lokale Einstellungen**: `~/.olenro/settings.json` (gerätebezogene UI-Einstellungen)
- **Backups**: `~/.olenro/backups/` (automatisch rotiert, behält die 10 neuesten)
- **Skills**: `~/.olenro/skills/` (standardmäßig per Symlink mit den entsprechenden Apps verbunden)
- **Skill-Backups**: `~/.olenro/skill-backups/` (vor der Deinstallation automatisch erstellt, behält die 20 neuesten)

</details>

## Dokumentation

Ausführliche Anleitungen zu jeder Funktion finden Sie im **[Benutzerhandbuch](docs/user-manual/en/README.md)** — es deckt Anbieterverwaltung, MCP/Prompts/Skills, Proxy & Failover und mehr ab.

## Schnellstart

### Grundlegende Verwendung

1. **Anbieter hinzufügen**: Klicken Sie auf „Add Provider" → Wählen Sie ein Preset oder erstellen Sie eine eigene Konfiguration
2. **Anbieter wechseln**:
   - Hauptoberfläche: Anbieter auswählen → auf „Enable" klicken
   - System-Tray: Anbietername direkt anklicken (sofort wirksam)
3. **Wirksam werden**: Starten Sie Ihr Terminal oder das entsprechende CLI-Werkzeug neu, um die Änderungen anzuwenden (Claude Code erfordert keinen Neustart)
4. **Zurück zum Offiziellen**: Fügen Sie ein „Official Login"-Preset hinzu, starten Sie das CLI-Werkzeug neu und folgen Sie dann seinem Login-/OAuth-Vorgang

### MCP, Prompts, Skills & Sessions

- **MCP**: Klicken Sie auf die Schaltfläche „MCP" → Server über Vorlagen oder eigene Konfiguration hinzufügen → Synchronisierung pro App umschalten
- **Prompts**: Klicken Sie auf „Prompts" → Presets mit dem Markdown-Editor erstellen → Aktivieren, um mit den Live-Dateien zu synchronisieren
- **Skills**: Klicken Sie auf „Skills" → GitHub-Repositorys durchsuchen → mit einem Klick in allen Apps installieren
- **Sessions**: Klicken Sie auf „Sessions" → Gesprächsverlauf über alle Apps hinweg durchsuchen, suchen und wiederherstellen

> **Hinweis**: Beim Erststart können Sie bestehende CLI-Werkzeug-Konfigurationen manuell als Standardanbieter importieren.

## Download & Installation

### Systemanforderungen

- **Windows**: Windows 10 und höher
- **macOS**: macOS 12 (Monterey) und höher
- **Linux**: Ubuntu 22.04+ / Debian 11+ / Fedora 34+ und andere gängige Distributionen

### Windows-Nutzer

Laden Sie das neueste Installationsprogramm `Olenro-v{version}-Windows.msi` oder die portable Version `Olenro-v{version}-Windows-Portable.zip` von der Seite [Releases](../../releases) herunter.

### macOS-Nutzer

**Methode 1: Installation über Homebrew (empfohlen)**

```bash
brew install --cask olenro
```

Aktualisieren:

```bash
brew upgrade --cask olenro
```

**Methode 2: Manueller Download**

Laden Sie `Olenro-v{version}-macOS.dmg` (empfohlen) oder `.zip` von der Seite [Releases](../../releases) herunter.

> **Hinweis**: Olenro für macOS ist von Apple code-signiert und notarisiert. Sie können es direkt installieren und öffnen.

### Arch-Linux-Nutzer

**Installation über paru (empfohlen)**

```bash
paru -S olenro-bin
```

### Linux-Nutzer

Laden Sie den neuesten Linux-Build von der Seite [Releases](../../releases) herunter:

- `Olenro-v{version}-Linux.deb` (Debian/Ubuntu)
- `Olenro-v{version}-Linux.rpm` (Fedora/RHEL/openSUSE)
- `Olenro-v{version}-Linux.AppImage` (universell)

> **Flatpak**: Nicht in den offiziellen Releases enthalten. Sie können es selbst aus dem `.deb` bauen — eine Anleitung finden Sie unter [`flatpak/README.md`](flatpak/README.md).

<details>
<summary><strong>Architekturüberblick</strong></summary>

### Designprinzipien

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (React + TS)                    │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │ Components  │  │    Hooks     │  │  TanStack Query  │    │
│  │   (UI)      │──│ (Bus. Logic) │──│   (Cache/Sync)   │    │
│  └─────────────┘  └──────────────┘  └──────────────────┘    │
└────────────────────────┬────────────────────────────────────┘
                         │ Tauri IPC
┌────────────────────────▼────────────────────────────────────┐
│                  Backend (Tauri + Rust)                     │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │  Commands   │  │   Services   │  │  Models/Config   │    │
│  │ (API Layer) │──│ (Bus. Layer) │──│     (Data)       │    │
│  └─────────────┘  └──────────────┘  └──────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

**Kern-Designmuster**

- **SSOT** (Single Source of Truth): Alle Daten werden in `~/.olenro/olenro.db` (SQLite) gespeichert
- **Zweischichtiger Speicher**: SQLite für synchronisierbare Daten, JSON für gerätebezogene Einstellungen
- **Bidirektionale Synchronisierung**: Schreiben in Live-Dateien beim Umschalten, Backfill aus den Live-Dateien beim Bearbeiten des aktiven Anbieters
- **Atomare Schreibvorgänge**: Das Muster aus temporärer Datei + Umbenennen verhindert die Beschädigung von Konfigurationen
- **Nebenläufigkeitssicher**: Eine durch Mutex geschützte Datenbankverbindung vermeidet Race Conditions
- **Geschichtete Architektur**: Klare Trennung (Commands → Services → DAO → Database)

**Schlüsselkomponenten**

- **ProviderService**: Anbieter-CRUD, Umschaltung, Backfill, Sortierung
- **McpService**: Verwaltung von MCP-Servern, Import/Export, Synchronisierung von Live-Dateien
- **ProxyService**: Lokaler Proxy-Modus mit Hot-Switching und Formatkonvertierung
- **SessionManager**: Durchsuchen des Gesprächsverlaufs über alle unterstützten Apps hinweg
- **ConfigService**: Konfigurations-Import/-Export, Backup-Rotation
- **SpeedtestService**: Messung der Latenz von API-Endpunkten

</details>

<details>
<summary><strong>Entwicklungsleitfaden</strong></summary>

### Umgebungsanforderungen

- Node.js 18+
- pnpm 8+
- Rust 1.85+
- Tauri CLI 2.8+

### Entwicklungsbefehle

```bash
# Abhängigkeiten installieren
pnpm install

# Entwicklungsmodus (Hot Reload)
pnpm dev

# Typprüfung
pnpm typecheck

# Code formatieren
pnpm format

# Codeformatierung prüfen
pnpm format:check

# Frontend-Unit-Tests ausführen
pnpm test:unit

# Tests im Watch-Modus ausführen (für die Entwicklung empfohlen)
pnpm test:unit:watch

# Anwendung bauen
pnpm build

# Debug-Version bauen
pnpm tauri build --debug
```

### Entwicklung des Rust-Backends

```bash
cd src-tauri

# Rust-Code formatieren
cargo fmt

# Clippy-Prüfungen ausführen
cargo clippy

# Backend-Tests ausführen
cargo test

# Bestimmte Tests ausführen
cargo test test_name

# Tests mit dem Feature test-hooks ausführen
cargo test --features test-hooks
```

### Testleitfaden

**Frontend-Tests**:

- Verwendet **vitest** als Test-Framework
- Verwendet **MSW (Mock Service Worker)**, um Tauri-API-Aufrufe zu mocken
- Verwendet **@testing-library/react** für Komponententests

**Tests ausführen**:

```bash
# Alle Tests ausführen
pnpm test:unit

# Watch-Modus (automatische erneute Ausführung)
pnpm test:unit:watch

# Mit Coverage-Bericht
pnpm test:unit --coverage
```

### Tech-Stack

**Frontend**: React 18 · TypeScript · Vite · TailwindCSS 3.4 · TanStack Query v5 · react-i18next · react-hook-form · zod · shadcn/ui · @dnd-kit

**Backend**: Tauri 2.8 · Rust · serde · tokio · thiserror · tauri-plugin-updater/process/dialog/store/log

**Testing**: vitest · MSW · @testing-library/react

</details>

<details>
<summary><strong>Projektstruktur</strong></summary>

```
├── src/                        # Frontend (React + TypeScript)
│   ├── components/
│   │   ├── providers/          # Anbieterverwaltung
│   │   ├── mcp/                # MCP-Panel
│   │   ├── prompts/            # Prompts-Verwaltung
│   │   ├── skills/             # Skills-Verwaltung
│   │   ├── sessions/           # Session Manager
│   │   ├── proxy/              # Proxy-Modus-Panel
│   │   ├── openclaw/           # OpenClaw-Konfigurationspanels
│   │   ├── settings/           # Einstellungen (Terminal/Backup/About)
│   │   ├── deeplink/           # Deep-Link-Import
│   │   ├── env/                # Verwaltung von Umgebungsvariablen
│   │   ├── universal/          # App-übergreifende Konfiguration
│   │   ├── usage/              # Nutzungsstatistik
│   │   └── ui/                 # shadcn/ui-Komponentenbibliothek
│   ├── hooks/                  # Eigene Hooks (Geschäftslogik)
│   ├── lib/
│   │   ├── api/                # Tauri-API-Wrapper (typsicher)
│   │   └── query/              # TanStack-Query-Konfiguration
│   ├── locales/                # Übersetzungen (zh/en/ja)
│   ├── config/                 # Presets (providers/mcp)
│   └── types/                  # TypeScript-Definitionen
├── src-tauri/                  # Backend (Rust)
│   └── src/
│       ├── commands/           # Tauri-Befehlsschicht (nach Domäne)
│       ├── services/           # Geschäftslogikschicht
│       ├── database/           # SQLite-DAO-Schicht
│       ├── proxy/              # Proxy-Modul
│       ├── session_manager/    # Sitzungsverwaltung
│       ├── deeplink/           # Deep-Link-Verarbeitung
│       └── mcp/                # MCP-Synchronisierungsmodul
├── tests/                      # Frontend-Tests
└── assets/                     # Screenshots & Partnerressourcen
```

</details>

## Mitwirken

Issues und Vorschläge sind willkommen!

Bitte stellen Sie vor dem Einreichen von PRs Folgendes sicher:

- Typprüfung besteht: `pnpm typecheck`
- Formatprüfung besteht: `pnpm format:check`
- Unit-Tests bestehen: `pnpm test:unit`

Eröffnen Sie für neue Funktionen bitte vor dem Einreichen eines PR ein Issue zur Diskussion. PRs für Funktionen, die nicht gut zum Projekt passen, können geschlossen werden.

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=lianhr12/olenro&type=Date)](https://www.star-history.com/#lianhr12/olenro&Date)

## Lizenz

MIT © Jason Young
