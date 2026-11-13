# Sicherheitsrichtlinie

eX-plorer arbeitet mit deinen Dateien. Deshalb sind Sicherheitsfragen hier wichtig,
und Hinweise sind willkommen.

## Unterstützte Versionen

Sicherheitskorrekturen gibt es für die **neueste veröffentlichte Version**
(siehe [Releases](https://github.com/theslasher5g/eX-plorer/releases)). Ältere
Versionen bekommen keine Korrekturen; bitte aktualisiere zuerst.

| Version | Unterstützt |
|---|---|
| neueste 0.x | ✅ |
| ältere | ❌ |

## Eine Schwachstelle melden

**Bitte erstelle für Sicherheitsprobleme kein öffentliches Issue.**

1. Öffne den Reiter **Security** dieses Repositorys und wähle
   **„Report a vulnerability“** (privates Melden über GitHub Security Advisories).
2. Beschreibe, was du gefunden hast, wie man es nachstellt (Windows-Version,
   App-Version, Schritte) und was passieren kann.
3. Du bekommst eine erste Antwort in der Regel **innerhalb von 7 Tagen**. Sobald
   das Problem bestätigt und behoben ist, wird es im Changelog genannt (auf
   Wunsch mit Namensnennung).

Ist das private Melden bei dir nicht möglich, eröffne ein Issue **ohne Details**
mit dem Hinweis „Sicherheitsproblem, bitte Kontakt aufnehmen“; dann klären wir
den Weg dorthin.

## Was als Schwachstelle zählt

Zum Beispiel:

- Zugriff auf Dateien oder Pfade, den die App nicht beabsichtigt (Pfad-Umgehung
  über `..`, Junctions, Symlinks, Alternate Data Streams, reservierte Namen,
  lange Pfade).
- Automatisches Ausführen oder Öffnen von Dateien ohne ausdrückliche Handlung.
- Ausbruch aus der Oberfläche: Skriptausführung über Dateinamen oder Vorschauen
  (XSS), Umgehen der Content-Security-Policy, Nachladen von Remote-Inhalten.
- Abruf von OneDrive-Cloud-Dateien („Hydration“) oder Zugriff auf Netzwerkpfade,
  den die App nur auf ausdrückliche Handlung vornehmen soll.
- Fehler in der Rechteprüfung der Tauri-Commands (siehe `src-tauri/capabilities`).
- Abstürze oder Hänger durch manipulierte Ordner (Denial of Service) mit
  realistischem Auslöser.

Nicht dazu zählen: Probleme, die Administratorrechte oder bereits ausgeführten
Schadcode auf dem Rechner voraussetzen, sowie Fehler in Drittsoftware (7-Zip,
Git …), deren Menüeinträge die App nur anzeigt.

## Wie die App abgesichert ist

Die verbindlichen Regeln stehen im [Handbuch](docs/HANDBUCH.md#sicherheit). Kurz:

- **Minimale Rechte.** Es gibt keine Tauri-Plugins für Dateisystem oder Shell;
  die Capabilities erlauben nur Fenstersteuerung und Fenster-Ereignisse. Alles
  andere läuft über eigene Commands, die **jeden Pfad im Backend prüfen**.
- **Strikte CSP**, keine Remote-Inhalte, kein `eval`, Dateinamen nie als HTML.
- **Nichts wird automatisch geöffnet oder ausgeführt**; Programme (exe, bat, ps1 …)
  nur nach ausdrücklicher Bestätigung.
- **Nur Metadaten beim Auflisten.** OneDrive-Platzhalter werden nie gelesen oder
  heruntergeladen, Netzwerkpfade haben Zeitlimits und keine automatische Tiefensuche.
- **Kein Administrator nötig**, keine Anmeldedaten in der App (Windows fragt selbst).
- `unsafe` nur in kleinen Windows-Modulen, `unwrap()`/`expect()` per Clippy-Lint
  verboten, `cargo clippy -- -D warnings` und `cargo audit` laufen in der CI.

> Die Installer sind **nicht signiert**. Windows SmartScreen kann deshalb warnen.
> Prüfe die SHA-256-Summen aus `SHA256SUMS.txt` am jeweiligen Release.

Dies ist ein Hobbyprojekt ohne Gewähr; eine unabhängige Sicherheitsprüfung hat
nicht stattgefunden.
