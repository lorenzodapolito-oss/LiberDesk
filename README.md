# LiberDesk

Gestisce le **app portabili** di una cartella (anche su chiavetta) e le tiene **aggiornate**, come LiberKey.
Programma Windows scritto in Rust (egui), un solo `.exe`, nessuna installazione.

## Come si usa
1. Metti `LiberDesk.exe` dove vuoi (anche sulla chiavetta). Accanto crea la cartella `Apps`.
2. Dentro `Apps` metti le app portabili, **una cartella per app** (es. `Apps\Notepad++`), oppure **cartelle di categoria** che contengono più programmi (es. `Apps\Internet\Firefox`, `Apps\Multimedia\VLC`): il nome della cartella diventa la categoria. Compaiono da sole nell'elenco.
3. Doppio clic su **Avvia** per aprirle.
4. All'avvio LiberDesk controlla se ci sono versioni nuove delle app che conosce, le scarica e le installa.

## Aggiornamenti
- Solo le app del catalogo interno si aggiornano (vedi `src/catalog.rs`); le altre si gestiscono e si avviano soltanto.
- La versione nuova viene cercata nelle release di GitHub del progetto.
- Prima di sostituire i file fa una **copia di sicurezza** in `Apps\_backup` (ne tiene due per app); se qualcosa va storto
  ripristina la versione precedente.
- **I tuoi dati restano**: cartelle `Data`, `Settings`, `Config`, `Profile`, `portable` e i file di impostazioni noti
  di ogni app non vengono toccati.
- Si possono spegnere il controllo all'avvio e l'installazione automatica (il pulsante «Aggiorna» resta).
- Sono supportati pacchetti `.zip`, `.7z` e file `.exe` singoli.

## Compilare
```
cargo build --release
cargo test
```
Su Windows basta `costruisci-liberdesk.cmd` (scarica, compila e crea `dist\LiberDesk-portable\LiberDesk.exe`).

## Aggiungere un'app al catalogo
In `src/catalog.rs` una riga `gh!(nome, categoria, [nomi cartella], eseguibile, [file da tenere], "utente/repo", "modello del file")`.
Il modello è un'espressione regolare sul nome del file da scaricare nella release.
