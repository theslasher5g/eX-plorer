# eX-plorer

Ein moderner Datei-Explorer für Windows 10/11 im Stil des macOS Finder.
Backend in Rust mit Tauri 2, Oberfläche in Svelte 5 + TypeScript.

> **Stand: Phase 2 von 7.** Fenster, Theme, Seitenleiste, Tabs, Ordner
> navigieren, Dateioperationen (Kopieren, Ausschneiden, Einfügen, Löschen,
> Umbenennen, Neuer Ordner), Drag & Drop und Live-Aktualisierung. Echte Icons,
> weitere Ansichten, Baumansicht, OneDrive-Status und Netzwerk folgen (siehe
> [Fahrplan](#fahrplan)).

Das Aussehen folgt dem Design „Lichtung“: Die Tokens (Farben, Radien, Schatten,
Bewegung) stehen in `src/app.css`, hell und dunkel folgen automatisch dem System.

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

## Bedienung

| Aktion | Eingabe |
| --- | --- |
| Ordner öffnen | Doppelklick oder Eingabetaste |
| Zurück / Vor | Alt+Links / Alt+Rechts, Maustasten 4/5 |
| Übergeordneter Ordner | Alt+Hoch oder Rücktaste |
| Pfad eintippen | Strg+L oder F4, oder Klick rechts neben den Pfad |
| Im Ordner suchen | Strg+F |
| Neuer Tab / Tab schließen / Tab wechseln | Strg+T / Strg+W oder Mittelklick / Strg+Tab |
| Aktualisieren | F5 |
| Auswahl | Klick, Strg+Klick, Umschalt+Klick, Pfeiltasten, Strg+A |
| Sortieren, versteckte Elemente anzeigen | Spaltenüberschrift oder Sortier-Menü in der Werkzeugleiste |
| Kopieren / Ausschneiden / Einfügen | Strg+C / Strg+X / Strg+V (auch mit dem Explorer und anderen Programmen) |
| Löschen | Entf (Papierkorb, mit Rückfrage), Umschalt+Entf (endgültig, mit Rückfrage) |
| Umbenennen | F2; Eingabe übernimmt, Esc verwirft, Wegklicken übernimmt |
| Neuer Ordner | Strg+Umschalt+N oder Knopf in der Werkzeugleiste |
| Verschieben / Kopieren per Maus | Ziehen auf Ordner, Tab, Pfadabschnitt oder Seitenleiste; Strg erzwingt Kopieren, Umschalt Verschieben |
| In andere Programme ziehen | Ausgewählte Dateien aus der Liste ziehen |

## Aufbau

```
src-tauri/
  crates/core/        Plattformneutraler Kern, ohne Tauri testbar
    src/path/         Pfadvalidierung und -normalisierung
    src/listing/      Ordner auflisten (Windows: FindFirstFileExW)
    src/ops/          Planung von Kopieren/Verschieben/Löschen/Umbenennen, Neuer Ordner
    src/watch.rs      Ordner-Watcher (notify), gebündelte Meldungen
    src/winerr.rs     HRESULT/Win32-Fehlercodes → Fehlerarten
  src/
    commands/         Tauri-Commands (einziger Weg vom Frontend ins System)
    shell/            Windows-Shell-Aufrufe: bekannte Ordner, Laufwerke,
                      IFileOperation, Zwischenablage (CF_HDROP), OLE-Drag, Tastenstatus
    state.rs          Fenstermaterial (Mica)
    error.rs          Fehler mit stabilen Codes fürs Frontend
  capabilities/       Tauri-Berechtigungen des Fensters
src/
  lib/api/            Typisierte Backend-Schnittstelle, Demo-Backend für den Browser
  lib/stores/         Ordneransicht (Pane), Tabs, Dateioperationen, Drag & Drop,
                      Toast und Bestätigungsdialog
  lib/components/     Titelleiste mit Tabs, Seitenleiste, Werkzeugleiste, Pfadleiste,
                      Sortier-Menü, Liste, Statusleiste
  app.css             Design-Tokens und gemeinsame Bausteine
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
- **Nichts wird ausgeführt.** Ein Doppelklick öffnet bisher ausschließlich Ordner.
- **Operationen sind doppelt abgesichert.** Der Kern plant (`ops`): keine
  Laufwerks-/Freigabe-Wurzeln, kein Ordner in sich selbst, doppelte und
  verschachtelte Auswahl wird zusammengefasst, Obergrenze von 20.000 Elementen.
  Ausgeführt wird über die Windows-Shell, die Papierkorb und Konflikte kennt.
- **Löschen fragt immer nach.** Wo es keinen Papierkorb gibt (Netzlaufwerk, USB,
  UNC), sagt der Dialog das deutlich; die Shell fragt zusätzlich selbst.
- **Strikte CSP**, keine Remote-Inhalte. Einzige Lockerung: `style-src
  'unsafe-inline'`, weil Svelte Inline-Styles für die virtualisierte Liste setzt.
- **Dateinamen** werden nur als Text gerendert, nie über `{@html}` oder `innerHTML`.
- `unsafe` steht nur in `crates/core/src/listing/win.rs` und
  `src-tauri/src/shell/`. `unwrap()`/`expect()` sind per Clippy-Lint verboten.
- Das Programm braucht keine Admin-Rechte.

## Bekannte Einschränkungen

- **Phase 2 wurde auf echtem Windows noch nicht getestet.** Die
  Dateioperationen, die Zwischenablage und der Drag in andere Programme
  greifen tief in Windows ein und lassen sich nur dort prüfen. Kompiliert und
  statisch geprüft sind sie, die Abläufe der Oberfläche liefen im Browser gegen
  ein Demo-Backend. Bitte besonders testen: Strg+C im Explorer und Strg+V hier
  (und umgekehrt), Dateien in den Explorer ziehen, Dateien aus dem Explorer
  hereinziehen.
- **Fortschritt und Konflikte zeigt Windows an**, nicht die App: Bei großen
  Vorgängen erscheint der gewohnte Windows-Dialog, ebenso bei Namenskonflikten.
  Eine eigene Fortschrittskarte gibt es nicht.
- **Kein Rückgängig in der App.** Gelöschtes lässt sich im Papierkorb
  wiederherstellen; Strg+Z gibt es nicht.
- **Drops aus anderen Programmen** werden vom Fenster als Kopie gemeldet. Ob
  daraus Kopieren oder Verschieben wird, entscheidet die App: gleiches Laufwerk
  verschiebt, anderes kopiert, Strg und Umschalt überstimmen das. Der Explorer
  löscht die Quelle deshalb nicht selbst.
- **Live-Aktualisierung** beobachtet nur die Ordner der offenen Tabs, nicht
  rekursiv und nicht auf Netzlaufwerken (dort hilft F5). Sehr viele schnelle
  Änderungen werden gebündelt, die Anzeige kann um Sekundenbruchteile nachhängen.
- **Pfade werden in normaler Form an die Shell gegeben**, Pfade über 260 Zeichen
  funktionieren nur, wenn Windows lange Pfade aktiviert hat.
- **Es gibt nur die Listenansicht**; Kacheln, Spalten und Baum aus dem Design
  folgen, ebenso Einstellungen und der Netzwerk-Eintrag der Seitenleiste.
- **Ein Doppelklick öffnet nur Ordner**, keine Dateien.
- **Symbole sind gezeichnet, nicht von Windows**: Ordnersymbol und farbige
  Typ-Kürzel aus dem Design. Echte Windows-Icons und Thumbnails kommen in
  Phase 3; die Typ-Spalte leitet sich bis dahin aus der Endung ab.
- **Tabs** merken sich Pfad, Verlauf, Sortierung und Auswahl, aber nicht die
  Scrollposition, und werden beim Beenden nicht gespeichert.
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
- **Getestet** wurde bisher: Kernlogik per Unit-Tests (u. a. Pfadvalidierung,
  Operationsplanung, Watcher), Windows-Code per Cross-Kompilierprüfung unter
  Linux und in der CI auf Windows, Oberfläche im Browser gegen das
  Demo-Backend (auch mit 100.000 Einträgen). Ein Test der fertigen App auf
  echtem Windows steht aus.

## Fahrplan

1. ✅ Projekt-Setup, Fenster, Theme, Seitenleiste, Ordner auflisten und navigieren
2. ✅ Dateioperationen, Drag & Drop, Tabs, Live-Aktualisierung
3. Icons, Thumbnails, Kontextmenü, Schnellvorschau
4. Baumansicht mit Limits, Abbruch und Export
5. OneDrive-Integration
6. Netzwerklaufwerke
7. Feinschliff: Performance, Tests, Sicherheitsprüfung, Installer
