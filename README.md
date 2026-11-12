# eX-plorer

Ein moderner Datei-Explorer für Windows 10/11 im Stil des macOS Finder.
Backend in Rust mit Tauri 2, Oberfläche in Svelte 5 + TypeScript.

> **Stand: Phase 1 von 7.** Fenster, Theme, Seitenleiste, Ordner auflisten und
> navigieren. Dateioperationen, Tabs, echte Icons, Baumansicht, OneDrive-Status
> und Netzwerk folgen in den nächsten Phasen (siehe [Fahrplan](#fahrplan)).

## Voraussetzungen

- Windows 10 (ab 1809) oder Windows 11
- [Rust](https://rustup.rs) (stable, MSVC-Toolchain) samt „Desktopentwicklung mit C++“
  aus den Visual Studio Build Tools
- [Node.js](https://nodejs.org) 22 oder neuer
- WebView2-Laufzeit (unter Windows 11 vorinstalliert)

## Starten und bauen

```bash
npm install
```

Entwicklungsmodus mit Hot-Reload:

```bash
npm run tauri dev
```

Release-Build mit Installern (NSIS und MSI) unter `src-tauri/target/release/bundle/`:

```bash
npm run tauri build
```

Nur die Oberfläche im Browser, mit erfundenen Demo-Daten und ohne Zugriff auf
das Dateisystem (praktisch für Design-Arbeit, auch unter Linux/macOS):

```bash
npm run dev
```

## Prüfungen

```bash
npm run check
```

Im Ordner `src-tauri`:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

```bash
cargo audit
```

Dieselben Schritte laufen in GitHub Actions auf `windows-latest`
(`.github/workflows/ci.yml`); der fertige Installer hängt als Artefakt am Lauf.

## Bedienung (Phase 1)

| Aktion | Eingabe |
| --- | --- |
| Ordner öffnen | Doppelklick oder Eingabetaste |
| Zurück / Vor | Alt+Links / Alt+Rechts, Maustasten 4/5 |
| Übergeordneter Ordner | Alt+Hoch oder Rücktaste |
| Pfad eintippen | Strg+L oder F4, oder Klick rechts neben den Pfad |
| Im Ordner filtern | Strg+F |
| Aktualisieren | F5 |
| Auswahl | Klick, Strg+Klick, Umschalt+Klick, Pfeiltasten, Strg+A |
| Sortieren | Klick auf eine Spaltenüberschrift |

## Aufbau

```
src-tauri/
  crates/core/        Plattformneutraler Kern, ohne Tauri testbar
    src/path/         Pfadvalidierung und -normalisierung
    src/listing/      Ordner auflisten (Windows: FindFirstFileExW)
  src/
    commands/         Tauri-Commands (einziger Weg vom Frontend ins System)
    shell/            Windows-Shell-Aufrufe (bekannte Ordner, Laufwerke)
    state.rs          Fenstermaterial (Mica)
    error.rs          Fehler mit stabilen Codes fürs Frontend
  capabilities/       Tauri-Berechtigungen des Fensters
src/
  lib/api/            Typisierte Backend-Schnittstelle, Demo-Backend für den Browser
  lib/stores/         Zustand einer Ordneransicht
  lib/components/     Seitenleiste, Werkzeugleiste, Pfadleiste, Liste, Statusleiste
  lib/i18n/           Texte (derzeit Deutsch)
```

## Sicherheit

- **Kein Dateisystemzugriff im Frontend.** Es gibt keine Tauri-Plugins für
  Dateisystem oder Shell. Die Capabilities erlauben nur Fenstersteuerung und
  Fenster-Events; alles andere läuft über eigene Commands.
- **Jeder Pfad wird im Backend geprüft** (`crates/core/src/path`): absolut,
  keine `..`-Komponenten, keine Geräte-Namensräume (`\\.\`, `\\?\`), keine
  Alternate Data Streams, keine reservierten Namen (CON, NUL, COM1 …), keine
  Komponenten mit Punkt/Leerzeichen am Ende, Längenbegrenzung. Zugriffe nutzen
  danach die `\\?\`-Form, damit Windows den geprüften Pfad nicht umdeutet.
- **Auflisten liest nur Metadaten.** Keine Datei wird geöffnet; OneDrive-
  Platzhalter werden dadurch nicht heruntergeladen.
- **Nichts wird ausgeführt.** In Phase 1 öffnet ein Doppelklick ausschließlich Ordner.
- **Strikte CSP**, keine Remote-Inhalte. Einzige Lockerung: `style-src
  'unsafe-inline'`, weil Svelte Inline-Styles für die virtualisierte Liste setzt.
- **Dateinamen** werden nur als Text gerendert, nie über `{@html}` oder `innerHTML`.
- `unsafe` steht nur in `crates/core/src/listing/win.rs` und
  `src-tauri/src/shell/`. `unwrap()`/`expect()` sind per Clippy-Lint verboten.
- Das Programm braucht keine Admin-Rechte.

## Bekannte Einschränkungen

- **Phase 1 öffnet keine Dateien** und kann nichts kopieren, verschieben oder
  löschen. Es gibt nur die Listenansicht und noch keine Tabs.
- **Symbole sind Platzhalter** (Lucide). Echte Windows-Icons und Thumbnails
  kommen in Phase 3; die Typ-Spalte leitet sich bis dahin aus der Endung ab.
- **Keine Live-Aktualisierung**: Änderungen im Ordner erscheinen erst nach F5.
- **Netzlaufwerke** werden in der Seitenleiste angezeigt, aber noch ohne
  Timeout und Offline-Status geöffnet. Ein nicht erreichbares Laufwerk friert
  die Oberfläche nicht ein, die Anzeige wartet aber, bis Windows aufgibt.
- **OneDrive** wird bisher nur über Umgebungsvariablen erkannt, ohne Sync-Status.
- **Streng geprüfte Pfade**: Ordner, deren Name auf Punkt oder Leerzeichen
  endet oder ein reservierter Gerätename ist (z. B. von WSL angelegt), lassen
  sich nicht öffnen. Dateinamen mit ungültigem UTF-16 werden mit
  Ersatzzeichen angezeigt.
- **Mica** gibt es nur unter Windows 11; Windows 10 bekommt einen deckenden
  Hintergrund. Wegen der eigenen Titelleiste fehlt das Snap-Layout-Flyout beim
  Überfahren von „Maximieren“ (Win+Z und Ziehen an den Rand funktionieren).
- **Getestet** wurde bisher: Kernlogik per Unit-Tests, Windows-Code per
  Cross-Check und CI, Oberfläche im Browser gegen das Demo-Backend (auch mit
  100.000 Einträgen). Ein Test der fertigen App auf echtem Windows steht aus.

## Fahrplan

1. ✅ Projekt-Setup, Fenster, Theme, Seitenleiste, Ordner auflisten und navigieren
2. Dateioperationen, Drag & Drop, Tabs
3. Icons, Thumbnails, Kontextmenü, Schnellvorschau
4. Baumansicht mit Limits, Abbruch und Export
5. OneDrive-Integration
6. Netzwerklaufwerke
7. Feinschliff: Performance, Tests, Sicherheitsprüfung, Installer
