## Was ändert sich?

<!-- Kurz und konkret. Verweise auf ein Issue, falls es eins gibt. -->

## Wie wurde es geprüft?

- [ ] `npm run check`
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] Im Demo-Modus (`npm run dev`) ausprobiert
- [ ] Auf Windows ausprobiert (nur nötig bei Shell, Drag & Drop, OneDrive, Netzwerk, Installer)

## Sicherheit

- [ ] Neue Pfadeingaben werden im Backend mit `validate_path` geprüft
- [ ] Keine neuen Rechte in `capabilities`, kein `{@html}`, nichts wird automatisch geöffnet
- [ ] Nicht zutreffend

## Hinweise für die Prüfung

<!-- Screenshots/GIFs, offene Fragen, bekannte Grenzen. -->
