export const de = {
  'app.title': 'eX-plorer',

  'sidebar.favorites': 'Favoriten',
  'sidebar.onedrive': 'OneDrive',
  'sidebar.drives': 'Laufwerke',
  'place.desktop': 'Desktop',
  'place.documents': 'Dokumente',
  'place.downloads': 'Downloads',
  'place.pictures': 'Bilder',
  'place.onedrive': 'OneDrive',
  'drive.fixed': 'Lokaler Datenträger',
  'drive.removable': 'Wechseldatenträger',
  'drive.remote': 'Netzlaufwerk',
  'drive.cdrom': 'CD/DVD-Laufwerk',
  'drive.ram': 'RAM-Disk',
  'drive.unknown': 'Laufwerk',
  'drive.free': '{free} frei von {total}',

  'nav.back': 'Zurück (Alt+Links)',
  'nav.forward': 'Vor (Alt+Rechts)',
  'nav.up': 'Übergeordneter Ordner (Alt+Hoch)',
  'nav.refresh': 'Aktualisieren (F5)',
  'nav.path': 'Pfad',
  'nav.editPath': 'Pfad bearbeiten (Strg+L)',
  'nav.filter': 'In diesem Ordner filtern',
  'nav.showHidden': 'Versteckte Dateien anzeigen',
  'nav.hideHidden': 'Versteckte Dateien ausblenden',

  'window.minimize': 'Minimieren',
  'window.maximize': 'Maximieren',
  'window.restore': 'Verkleinern',
  'window.close': 'Schließen',

  'col.name': 'Name',
  'col.modified': 'Geändert',
  'col.type': 'Typ',
  'col.size': 'Größe',

  'type.folder': 'Ordner',
  'type.link': 'Verknüpfter Ordner',
  'type.file': 'Datei',
  'type.fileExt': '{ext}-Datei',

  'list.empty': 'Dieser Ordner ist leer.',
  'list.noMatch': 'Keine Treffer für „{filter}“.',
  'list.loading': 'Wird geladen …',

  'status.items': '{count} Elemente',
  'status.item': '1 Element',
  'status.selected': '{count} ausgewählt',
  'status.hidden': '{count} versteckt',

  'error.title': 'Ordner kann nicht angezeigt werden',
  'error.retry': 'Erneut versuchen',
  'error.not_found': 'Der Ordner wurde nicht gefunden.',
  'error.access_denied': 'Zugriff verweigert. Für diesen Ordner fehlen die Berechtigungen.',
  'error.not_a_directory': 'Der Pfad ist kein Ordner.',
  'error.invalid_path': 'Der Pfad ist ungültig oder nicht erlaubt.',
  'error.io': 'Beim Lesen des Ordners ist ein Fehler aufgetreten.',
  'error.internal': 'Ein unerwarteter Fehler ist aufgetreten.',
} as const;

export type MessageKey = keyof typeof de;
