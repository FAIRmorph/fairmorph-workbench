# P0.6 MVP – Geführte Projektinitialisierung

**Scope:** Standardisierte Svelte-5-Workbench-Bedienung für eine **neue** Projektidentität und den Import unveränderter Originaldateien. Sie verwendet die bestehenden Engine-Kommandos `init-project` und `import-resource` über die allowlisted Rust/Tauri-IPC. Dies ist **keine** Erweiterung der wissenschaftlichen Engine-Contracts und **keine** automatische Konfiguration/Bestätigung einer Entity-Pipeline.

## Ablauf

1. In der Workbench unter **Neues Projekt** den Projektnamen eingeben.
2. Das übergeordnete Verzeichnis mit dem nativen Ordnerdialog auswählen (oder einen absoluten Pfad eintragen).
3. Einen **noch nicht existierenden** Unterordnernamen angeben, z. B. `survey.fairmorph`.
4. **Projekt anlegen** ruft `init-project --project <neuer-pfad> --name <name>` auf. Die Engine erzeugt selbst `project.json`, SQLite-Katalog und Verzeichnisstruktur.
5. **Dateien auswählen** öffnet den nativen Dateiauswahldialog. Jede Originaldatei wird einzeln aufgeführt.
6. **Für jede Datei ausdrücklich eine technische Rolle wählen:** `events` oder `counts`. Es gibt **keine Voreinstellung** und keine Heuristik über Namen, CSV-Inhalt oder Dateiendung.
7. **Originaldateien importieren** ruft je gewählter Datei `import-resource --project <pfad> --source <originalpfad> --role <rolle>` auf. Die Workbench zeigt die SHA-256 aus der Engine-Antwort an.
8. Das Projekt ist jetzt in der bestehenden Ansicht geöffnet und kann **anschließend** konfiguriert werden.

## Grenzen des ersten vertikalen Schnitts

- Nur `events` und `counts` sind vorerst als **technische Ressourcenrollen** auswählbar. Sie sind keine bestätigten FAIRmorph-Entities und keine fachliche Freigabe. Die Rollenliste muss später unabhängig vom Datensatz erweiterbar werden.
- Kein automatisch erzeugtes Mapping, keine Pipeline-Generierung, keine wissenschaftlichen Schema-/Metadata-Approvals.
- **Kein automatischer RUN nach Import.** Eine vorher registrierte und explizit ausgewählte Pipeline bleibt Voraussetzung für `run-pipeline`. Das RUN-Feature der bestehenden Workbench bleibt unverändert.
- Der Projektordner darf an einem frei gewählten lokalen absoluten Pfad liegen. Ziel darf nicht bereits existieren. Elternordner muss existieren. Keine Löschung oder Überschreibung bestehender Projekte.
- Bei Fehlern während mehrerer Importe bleiben bereits erfolgreich registrierte Ressourcen erhalten; die UI markiert sie und erlaubt das erneute Importieren der noch offenen Dateien.
- Das Projektpackage enthält weiterhin ausschließlich Engine-erzeugte Daten und Konfigurationen, **keine ausführbaren Skripte**.
- Die Dateiauswahl nutzt die vorhandene Rust-Dependency `tauri-plugin-dialog`; kein externes JS-Plugin erforderlich.

## Manuelle Abnahme

1. Engine-Handshake ist erfolgreich und nennt `init-project`, `import-resource`.
2. Ein neues Projekt mit frei gewähltem Namen und Ordner wird erstellt. `project.json` enthält den Namen, eine UUID und leere Import-/RUN-Referenzen.
3. Datei auswählen: Ohne Rollenzuordnung bleibt Import deaktiviert.
4. Zwei Dateien explizit als `events` und `counts` zuordnen und importieren; danach werden zwei SHA-256 angezeigt.
5. Mit `fairmorph-engine verify-project --project <pfad>` die Integrität prüfen.
6. Überprüfen, dass die Originaldateien unverändert sind und das Projekt keinen Python-Code enthält.
7. Bei Auswahl eines existierenden Zielverzeichnisses oder einer fehlenden Quelle erscheint ein Fehler ohne Überschreiben.
8. Der RUN-Bereich startet **ohne registrierte Pipeline** keinen erfolgreichen RUN. Später wird eine Pipeline separat konfiguriert und ausgewählt.

**P0.6-Grenze:** Initialisierung und Ressourcenimport können unabhängig von einer fachlichen RUN-Abnahme geprüft werden; das ist keine Aussage über wissenschaftliche Konformität.
