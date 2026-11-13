# eX-plorer

Ein moderner Datei-Explorer für Windows 10/11 im Stil des macOS Finder.
Backend in Rust mit Tauri 2, Oberfläche in Svelte 5 + TypeScript.

> **Stand: Phase 5 von 7.** Fenster, Theme, Seitenleiste, Tabs, Navigation,
> Dateioperationen, Drag & Drop, Live-Aktualisierung, Windows-Icons, natives
> Kontextmenü, Schnellvorschau, Listen-, Kachel- und Baumansicht mit Limits,
> Einstellungen und die OneDrive-Integration. Netzwerklaufwerke folgen (siehe
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
| Datei öffnen | Doppelklick oder Eingabetaste (ausführbare Typen erst nach Rückfrage) |
| Schnellvorschau | Leertaste; Pfeiltasten wechseln die Datei, Bild auf/ab bei PDF-Seiten, Esc oder Leertaste schließen |
| Ansicht wechseln | Strg+1 Liste, Strg+2 Kacheln, Strg+3 Baum, oder der Umschalter in der Werkzeugleiste |
| Baum bedienen | Pfeil rechts/links klappt auf/zu, Eingabetaste öffnet, Klick auf „… und N weitere“ lädt nach |
| Einstellungen | Strg+, oder der Eintrag unten in der Seitenleiste |
| OneDrive: Behalten / Freigeben | Rechtsklick auf Dateien oder Ordner im OneDrive-Ordner: „Immer auf diesem Gerät behalten“ bzw. „Speicherplatz freigeben“ |
| Kontextmenü | Rechtsklick auf eine Datei, einen Ordner oder die leere Fläche (Umschalt+Rechtsklick zeigt erweiterte Einträge) |

## Aufbau

```
src-tauri/
  crates/core/        Plattformneutraler Kern, ohne Tauri testbar
    src/path/         Pfadvalidierung und -normalisierung
    src/listing/      Ordner auflisten (Windows: FindFirstFileExW)
    src/ops/          Planung von Kopieren/Verschieben/Löschen/Umbenennen, Neuer Ordner
    src/tree/         Baum-Scan mit Limits (Tiefe, gesamt, pro Ordner), abbrechbar
    src/settings.rs   Einstellungen mit erzwungenen Wertebereichen
    src/preview.rs    Vorschau-Regeln (Art, Limits) und sichere Text-Dekodierung
    src/imaging.rs    BGRA → PNG für Icons und Thumbnails
    src/exec.rs       Welche Dateitypen beim Öffnen eine Rückfrage brauchen
    src/attrs.rs      Cloud-Platzhalter, Sync-Zustand, Pin-Attribute
    src/onedrive.rs   OneDrive-Konten zusammenführen (Registry, Umgebung)
    src/watch.rs      Ordner-Watcher (notify), gebündelte Meldungen
    src/winerr.rs     HRESULT/Win32-Fehlercodes → Fehlerarten
  src/
    commands/         Tauri-Commands (einziger Weg vom Frontend ins System)
    shell/            Windows-Shell-Aufrufe: bekannte Ordner, Laufwerke,
                      IFileOperation, Zwischenablage (CF_HDROP), OLE-Drag, Tastenstatus,
                      Icons/Thumbnails, Kontextmenü (eigener Thread), Datei öffnen,
                      OneDrive (Konten, Pin/Unpin), Registry (nur lesend)
    protocol.rs       URI-Schema shellimg (Icons/Thumbnails als PNG)
    workers.rs        Arbeits-Pool für Bildanfragen (neueste zuerst, begrenzte Warteschlange)
    state.rs          Fenstermaterial (Mica)
    error.rs          Fehler mit stabilen Codes fürs Frontend
  capabilities/       Tauri-Berechtigungen des Fensters
src/
  lib/api/            Typisierte Backend-Schnittstelle, Demo-Backend für den Browser
  lib/stores/         Ordneransicht (Pane), Tabs, Dateioperationen, Drag & Drop,
                      Schnellvorschau, Toast und Bestätigungsdialog
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
- **Nichts wird automatisch geöffnet oder ausgeführt.** Dateien öffnen sich nur
  auf Doppelklick, Eingabetaste oder Menübefehl, und Typen, die Code ausführen
  können (exe, bat, ps1, msi, lnk, js …), erst nach ausdrücklicher Bestätigung.
  Die Liste dieser Typen steht im Backend (`exec.rs`, getestet).
- **OneDrive arbeitet nur mit Metadaten.** Der Sync-Zustand kommt aus den
  Verzeichniseinträgen (Attribute, Reparse-Tag, Cloud-Files-API
  `CfGetPlaceholderStateFromFindData`), „Behalten“ und „Freigeben“ setzen nur
  die Attribute PINNED bzw. UNPINNED; Herunterladen und Freigeben erledigt der
  OneDrive-Client. Die Registry wird nur gelesen, und das Setzen ist auf
  Pfade innerhalb der erkannten OneDrive-Ordner beschränkt (nicht auf deren
  Wurzel).
- **Cloud-Platzhalter werden nie gelesen.** Für „nur in der Cloud“-Dateien
  (OneDrive) erzeugt das Backend weder Icons noch Thumbnails noch Vorschauen;
  die Attribute werden vorher per Metadaten geprüft. Dasselbe gilt für Icons
  auf Netzwerkpfaden.
- **Vorschau ist begrenzt:** Bilder bis 40 MiB, PDF bis 64 MiB, Text die ersten
  256 KiB. SVG wird als Text gezeigt, nie als Bild, damit garantiert nichts
  ausgeführt wird. Text wird über `textContent` angezeigt, nie als HTML.
- **Operationen sind doppelt abgesichert.** Der Kern plant (`ops`): keine
  Laufwerks-/Freigabe-Wurzeln, kein Ordner in sich selbst, doppelte und
  verschachtelte Auswahl wird zusammengefasst, Obergrenze von 20.000 Elementen.
  Ausgeführt wird über die Windows-Shell, die Papierkorb und Konflikte kennt.
- **Löschen fragt immer nach.** Wo es keinen Papierkorb gibt (Netzlaufwerk, USB,
  UNC), sagt der Dialog das deutlich; die Shell fragt zusätzlich selbst.
- **Baum-Scan ist begrenzt und abbrechbar.** Tiefe, Gesamtzahl und Einträge
  pro Ordner werden im Backend erzwungen (auch gegen manipulierte Aufrufe),
  Symlinks und Junctions nie automatisch betreten, auf Netzlaufwerken wird
  automatisch nur eine Ebene gelesen. Der Speicherbedarf pro Ordner hängt
  nicht von dessen Größe ab.
- **Strikte CSP**, keine Remote-Inhalte. Lockerungen und ihr Grund:
  `style-src 'unsafe-inline'` (Svelte setzt Inline-Styles für die virtualisierte
  Liste), `script-src 'wasm-unsafe-eval'` (pdf.js nutzt WebAssembly für
  JPEG2000; erlaubt kein `eval`), `connect-src 'self'` und `worker-src 'self'
  blob:` (pdf.js lädt Schriften und seinen Worker aus der App),
  `img-src blob: http://shellimg.localhost` (Vorschau und Icons) und
  `font-src data: blob:` (von pdf.js erzeugte Schriften).
- **Dateinamen** werden nur als Text gerendert, nie über `{@html}` oder `innerHTML`.
- `unsafe` steht nur in `crates/core/src/listing/win.rs` und
  `src-tauri/src/shell/`. `unwrap()`/`expect()` sind per Clippy-Lint verboten.
- Das Programm braucht keine Admin-Rechte.

## Bekannte Einschränkungen

- **Phase 2 bis 5 wurden auf echtem Windows noch nicht getestet.** Dateioperationen,
  Zwischenablage, Drag in andere Programme, Icon-Gewinnung, das native
  Kontextmenü und das Öffnen von Dateien greifen tief in Windows ein und lassen
  sich nur dort prüfen. Kompiliert und statisch geprüft sind sie, die Abläufe
  der Oberfläche liefen im Browser gegen ein Demo-Backend. Bitte besonders
  testen: Strg+C im Explorer und Strg+V hier (und umgekehrt), Dateien in den
  Explorer und aus ihm ziehen, Rechtsklick (erscheint z. B. 7-Zip?), Icons und
  Schnellvorschau in einem Ordner mit vielen Bildern.
- **Kontextmenü:** Es erscheint das klassische Windows-Menü, wie unter „Weitere
  Optionen anzeigen“ in Windows 11; Einträge moderner Paket-Apps, die nur im
  neuen Menü stehen, fehlen teilweise. Ausschneiden, Kopieren, Einfügen,
  Löschen, Umbenennen und Öffnen führt die App selbst aus (damit Rückfragen
  und Anzeige stimmen), alles andere die Shell. Das Menü öffnet an der
  Mausposition; über die Menütaste ausgelöst erscheint es dort, wo der Zeiger
  steht. „Neu“-Einträge der leeren Fläche legen Dateien an, wählen sie aber
  nicht zum Umbenennen aus.
- **Icons:** Dateien zeigen das Windows-Icon bzw. Thumbnail (32 px, nur
  sichtbare Zeilen, drei Arbeits-Threads), solange es geladen wird oder wenn
  nichts erzeugt werden darf (Cloud, Netzwerk) das gezeichnete Typ-Symbol.
  Ordner behalten das gezeichnete Symbol des Designs. Das Backend rechnet
  vormultiplizierte Alphawerte zurück; bei teiltransparenten Randpixeln kann
  das minimal abweichen.
- **Thumbnails** erscheinen bisher nur als Listen-Icon und in der
  Schnellvorschau. Eine Kachelansicht, in der sie groß zur Geltung kommen,
  fehlt noch; auf Netzlaufwerken gibt es sie auch dort nicht automatisch.
- **Schnellvorschau:** Bilder, Text und PDF. Andere Typen zeigen Name, Art,
  Größe, Datum und, falls Windows eines liefert, ein großes Thumbnail. Keine
  Wiedergabe von Video oder Audio, bei PDF keine Textauswahl und keine Links.
  PDFs mit Schriften, die nur über Adobe-CMaps (v. a. CJK) definiert und nicht
  eingebettet sind, werden evtl. falsch dargestellt; die CMaps liegen nicht bei.
  Die App wächst durch pdf.js um rund 4 MB, geladen wird es erst beim ersten PDF.
- **Dateien auf Netzlaufwerken** werden für die Vorschau gelesen (auf
  ausdrückliche Handlung); ein Timeout für nicht erreichbare Server folgt mit
  der Netzwerk-Phase.
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
- **Baumansicht:** Sie zeigt und öffnet, aber sie bearbeitet nicht: Kopieren,
  Löschen, Umbenennen, Ziehen und die Schnellvorschau gibt es dort nicht
  (Rechtsklick zeigt das Windows-Menü, daraus wirkt nur „Öffnen“ in der App).
  Sie aktualisiert sich nicht live; ein Wechsel des Ordners, geänderte Limits
  oder F5 scannen neu, und dabei klappen von Hand geöffnete Ordner wieder zu.
  Sortiert wird immer nach Name (Ordner zuerst); Suche und Sortier-Menü wirken
  dort nicht. Beim Gesamtlimit liest der Scan ebenenweise, die oberen Ebenen
  sind also vollständig. Ein Ordner mit sehr vielen Einträgen wird trotz
  Limit einmal ganz durchgezählt (für „… und N weitere“), das kann bei
  Millionen Einträgen einige Sekunden dauern, ist aber abbrechbar.
- **Kachelansicht:** Thumbnails kommen von Windows in 256 px. Ordner zeigen das
  gezeichnete Symbol, keine Inhaltsvorschau.
- **Es fehlen** die Spaltenansicht und die Gruppierung aus den Anforderungen
  sowie der Netzwerk-Eintrag der Seitenleiste. Die Einstellungen umfassen
  bisher versteckte Dateien, Kachelgröße und die Baum-Limits; ein manueller
  Hell/Dunkel-Schalter fehlt (die App folgt Windows).
- **Die Ansicht gilt pro Tab** und wird nicht gespeichert; neue Tabs starten
  in der Liste.
- **Symbole sind gezeichnet, nicht von Windows**: Ordnersymbol und farbige
  Typ-Kürzel aus dem Design. Echte Windows-Icons und Thumbnails kommen in
  Phase 3; die Typ-Spalte leitet sich bis dahin aus der Endung ab.
- **Tabs** merken sich Pfad, Verlauf, Ansicht, Sortierung und Auswahl, aber nicht die
  Scrollposition, und werden beim Beenden nicht gespeichert.
- **Netzlaufwerke** werden in der Seitenleiste angezeigt, aber noch ohne
  Timeout und Offline-Status geöffnet. Ein nicht erreichbares Laufwerk friert
  die Oberfläche nicht ein, die Anzeige wartet aber, bis Windows aufgibt.
- **OneDrive:**
  - Konten kommen aus der Registry (`HKCU\Software\Microsoft\OneDrive\Accounts`)
    und den Umgebungsvariablen `OneDrive`, `OneDriveConsumer`,
    `OneDriveCommercial`, ohne Doppelte. Nicht erkannt werden zu Bibliotheken
    synchronisierte SharePoint-Ordner außerhalb des Konto-Ordners.
  - Der Status steht in der Typ-Spalte (Liste) bzw. als Eckmarke (Kacheln),
    nicht in der Baumansicht. „Wird synchronisiert“ gilt auch für normale
    Dateien im OneDrive-Ordner, die der Client noch nicht umgewandelt hat;
    versteckte und Systemdateien (desktop.ini) haben keinen Status. Pausiert
    OneDrive, bleibt ein Eintrag entsprechend lange „Synchronisiert …“.
  - Die beiden Aktionen kommen im Kontextmenü; hat OneDrive sie schon selbst
    im Menü (deutsch oder englisch beschriftet), fügt die App sie nicht ein.
    Bei anderer Windows-Sprache können sie doppelt erscheinen.
  - Bei Ordnern wird das Attribut am Ordner gesetzt; ob OneDrive es auf den
    bestehenden Inhalt anwendet, entscheidet der Client. Der Status ändert sich
    erst, wenn OneDrive reagiert hat.
  - Die Cloud-Files-API (`cldapi.dll`) gibt es ab Windows 10 1709.
- **Streng geprüfte Pfade**: Ordner, deren Name auf Punkt oder Leerzeichen
  endet oder ein reservierter Gerätename ist (z. B. von WSL angelegt), lassen
  sich nicht öffnen. Dateinamen mit ungültigem UTF-16 werden mit
  Ersatzzeichen angezeigt.
- **Mica** gibt es nur unter Windows 11; Windows 10 bekommt einen deckenden
  Hintergrund. Wegen der eigenen Titelleiste fehlt das Snap-Layout-Flyout beim
  Überfahren von „Maximieren“ (Win+Z und Ziehen an den Rand funktionieren).
- **Getestet** wurde bisher: Kernlogik per Unit-Tests (u. a. Pfadvalidierung,
  Operationsplanung, Sync-Zustand und OneDrive-Konten, Watcher, Baum-Limits mit 100.000 Dateien in einem Ordner
  und einer Link-Endlosschleife), Windows-Code per Cross-Kompilierprüfung unter
  Linux und in der CI auf Windows, Oberfläche im Browser gegen das
  Demo-Backend (auch mit 100.000 Einträgen). Ein Test der fertigen App auf
  echtem Windows steht aus.

## Fahrplan

1. ✅ Projekt-Setup, Fenster, Theme, Seitenleiste, Ordner auflisten und navigieren
2. ✅ Dateioperationen, Drag & Drop, Tabs, Live-Aktualisierung
3. ✅ Icons, Thumbnails, Kontextmenü, Schnellvorschau, Datei öffnen
4. ✅ Baumansicht mit Limits, Abbruch und Export; dazu Kachelansicht und Einstellungen
5. ✅ OneDrive-Integration
6. Netzwerklaufwerke
7. Feinschliff: Performance, Tests, Sicherheitsprüfung, Installer

Noch keiner Phase zugeordnet, aber in den Anforderungen und im Design
vorhanden: Spaltenansicht und Gruppierung.
