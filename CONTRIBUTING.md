# Mitmachen

Danke, dass du eX-plorer verbessern willst! Fehler, Ideen und Pull Requests sind
willkommen.

## Fehler melden und Ideen einbringen

- **Fehler:** Nutze die Vorlage „Fehlerbericht“. Hilfreich sind Windows-Version,
  App-Version (Einstellungen / Installer-Name), Schritte zum Nachstellen und
  ein Screenshot.
- **Ideen:** Vorlage „Funktionswunsch“. Beschreibe das Problem, nicht nur die Lösung.
- **Sicherheitsprobleme:** nicht öffentlich melden, siehe [SECURITY.md](SECURITY.md).

## Entwicklungsumgebung

Voraussetzungen und Befehle stehen im [README](README.md#selbst-bauen). Kurzfassung:

```bash
npm install
npm run tauri dev      # App mit Hot-Reload (Windows)
npm run dev            # nur die Oberfläche im Browser mit Demo-Daten (überall)
```

Der Kern (`src-tauri/crates/core`) ist plattformneutral und lässt sich auch unter
Linux/macOS testen: `cargo test -p explorer-core`.

## Vor dem Pull Request

```bash
npm run check                                    # svelte-check
cd src-tauri
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

Die CI führt dasselbe auf `windows-latest` aus.

## Regeln für den Code

Diese Punkte sind **verbindlich** (siehe auch [Sicherheit im Handbuch](docs/HANDBUCH.md#sicherheit)):

- Das Frontend greift **nie** direkt aufs Dateisystem zu; neue Funktionen sind
  Tauri-Commands, die **jeden Pfad** mit `validate_path` prüfen.
- Keine Tauri-Plugins für Dateisystem/Shell, Capabilities so klein wie möglich.
- Kein `{@html}`/`innerHTML` mit Dateinamen oder Dateiinhalten.
- Nichts öffnen oder ausführen ohne ausdrückliche Handlung des Benutzers.
- Beim Auflisten nur Metadaten lesen; Cloud-Platzhalter und Netzwerkpfade nie
  automatisch öffnen.
- `unsafe` nur in den kleinen Windows-API-Modulen (`src-tauri/src/shell`,
  `crates/core/src/listing/win.rs`), mit `// SAFETY:`-Kommentar.
- Kein `unwrap()`/`expect()` im Produktivcode, Fehler mit `thiserror`.
- Texte der Oberfläche gehören in `src/lib/i18n/de.ts`, nicht in die Komponenten.
- Neue API-Methoden stehen in `types.ts`, `tauri.ts` **und** `mock.ts` (Demo-Modus).

## Commits und Pull Requests

- Kleine, thematisch zusammenhängende Commits mit aussagekräftiger Nachricht.
- Beschreibe im PR, was sich ändert und wie du es geprüft hast. Änderungen, die
  nur unter Windows prüfbar sind (Shell, Drag & Drop, OneDrive, Netzwerk), bitte
  ausdrücklich so kennzeichnen.
- Neue Funktionen bekommen Tests (Kern: Rust-Tests; Oberfläche: Verhalten im
  Demo-Modus nachvollziehbar beschreiben).

## Verhaltensregeln

Es gilt der [Verhaltenskodex](CODE_OF_CONDUCT.md).
