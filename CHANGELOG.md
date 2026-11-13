# Änderungen

Alle nennenswerten Änderungen stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/), die Versionen folgen
[Semantic Versioning](https://semver.org/lang/de/): Solange die Hauptversion 0 ist,
können sich auch Nebenversionen noch ändern.

## [Unveröffentlicht]

## [0.1.0] - 2026-11-13

Erste öffentliche Version.

### Hinzugefügt

- **Oberfläche** im Stil des macOS Finder („Lichtung“): Hell/Dunkel (auch fest
  wählbar), Mica unter Windows 11, abgerundet, Segoe-UI-Schrift, virtualisierte
  Listen (getestet mit 100.000 Einträgen).
- **Ansichten:** Liste, Kacheln, Spalten (mit Vorschau wie im Finder) und
  Baumansicht mit Limits (Tiefe, Gesamtzahl, pro Ordner), Export als `tree`-Text.
- **Navigation:** Seitenleiste (Favoriten, OneDrive-Konten, Laufwerke, Netzwerk),
  Brotkrumen und getippte Pfade, Tabs, Verlauf, Maustasten Zurück/Vor.
- **Dateioperationen** über die Windows-Shell (`IFileOperation`): Kopieren,
  Ausschneiden, Einfügen, Verschieben, Umbenennen, Neuer Ordner, Löschen mit
  Papierkorb und Rückfrage; Zwischenablage kompatibel zum Explorer.
- **Drag & Drop** innerhalb der App (Ziel live markiert) und von/zu anderen Programmen.
- **Kontextmenü:** Windows baut das Menü auf (7-Zip, Git, „Senden an“ …), die App
  zeichnet es im eigenen Stil; „Weitere Optionen“ zeigt das native Menü.
- **Schnellvorschau** (Leertaste) für Bilder, Text und PDF (pdf.js lokal gebündelt),
  Windows-Icons und -Thumbnails asynchron.
- **Suche** im Ordner und allen Unterordnern (mit `*` und `?`), Tippsuche mit den
  ersten Buchstaben, Gruppieren (Name, Typ, Datum, Größe), Sortieren.
- **Gummiband-Auswahl** per Maus mit Autoscroll.
- **Favoriten** per Rechtsklick anheften, per Ziehen umsortieren.
- **OneDrive:** mehrere Konten, Statussymbole, „Immer behalten“/„Speicherplatz
  freigeben“ – ohne je Dateien herunterzuladen.
- **Netzlaufwerke:** UNC-Pfade, Status mit Zeitlimit, Anmeldung über den
  Windows-Dialog (keine Passwörter in der App).
- **Live-Aktualisierung** der Ordner (`notify`), Einstellungen (Erscheinungsbild,
  Endungen, Startansicht, Limits, Netzwerk-Zeitlimit).
- **Installer** (NSIS pro Benutzer, MSI), läuft ohne Administratorrechte.

### Sicherheit

- Minimale Tauri-Capabilities, kein Dateisystem-Plugin, strikte CSP.
- Jeder Pfad wird im Backend geprüft (Traversal, Junctions, reservierte Namen,
  Alternate Data Streams, lange Pfade).
- Nichts wird automatisch geöffnet oder ausgeführt; ausführbare Typen nur nach
  Bestätigung.

### Bekannte Einschränkungen

- Die Installer sind nicht signiert (SmartScreen-Warnung möglich).
- Vieles greift tief in Windows ein und wurde bisher nur von wenigen Personen auf
  echten Rechnern geprüft. Details im [Handbuch](docs/HANDBUCH.md#bekannte-einschränkungen).

[Unveröffentlicht]: https://github.com/theslasher5g/eX-plorer/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/theslasher5g/eX-plorer/releases/tag/v0.1.0
