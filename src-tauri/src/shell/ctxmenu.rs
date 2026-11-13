//! Das native Windows-Kontextmenü (`IContextMenu`), damit Erweiterungen wie
//! 7-Zip, „Öffnen mit“, OneDrive oder Git erscheinen.
//!
//! Das Menü läuft auf einem eigenen, dauerhaften Thread mit verstecktem
//! Fenster und Nachrichtenschleife. So blockiert es nie die Oberfläche, die
//! Nachrichten für Untermenüs (`IContextMenu2/3`) landen am richtigen Ort,
//! und Dialoge der Shell (z. B. „Eigenschaften“) behalten ihre Schleife.
//!
//! Das Menü wird **nicht** von Windows angezeigt: Das Backend baut es auf,
//! liest die Einträge aus (Text, Symbol, Untermenüs, Häkchen) und gibt sie als
//! Daten an die Oberfläche, die es im eigenen Stil zeichnet. Ein Klick führt
//! den Befehl über `IContextMenu` aus. „Weitere Optionen“ zeigt als Rückfall
//! das native Windows-Menü (für Erweiterungen, die sich selbst zeichnen).
//!
//! Befehle, die die App selbst besser kann (Ausschneiden, Kopieren, Einfügen,
//! Löschen, Umbenennen, Öffnen), werden nicht von der Shell ausgeführt,
//! sondern als Verb an die Oberfläche zurückgegeben.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::mem::size_of;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use explorer_core::imaging::to_base64;
use explorer_core::path::ValidPath;
use explorer_core::CoreError;
use serde::Serialize;
use windows::core::{w, Interface, HSTRING, PCSTR, PCWSTR, PSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    BHID_SFObject, BHID_SFUIObject, IContextMenu, IContextMenu2, IContextMenu3, IShellFolder,
    CMF_CANRENAME, CMF_EXTENDEDVERBS, CMF_NORMAL, CMIC_MASK_PTINVOKE, CMINVOKECOMMANDINFO,
    CMINVOKECOMMANDINFOEX, GCS_VERBW, SEE_MASK_UNICODE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW, GetCursorPos,
    GetMenuItemCount, GetMenuItemInfoW, GetMenuStringW, GetMessageW, GetSubMenu, InsertMenuW,
    PostMessageW, RegisterClassW, SetForegroundWindow, TrackPopupMenuEx, TranslateMessage, HMENU,
    MENUITEMINFOW, MFS_CHECKED, MFS_DEFAULT, MFS_DISABLED, MFS_GRAYED, MFT_SEPARATOR,
    MF_BYPOSITION, MF_STRING, MIIM_BITMAP, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_SUBMENU, MSG,
    SW_SHOWNORMAL, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_DRAWITEM, WM_INITMENUPOPUP,
    WM_MEASUREITEM, WM_MENUCHAR, WM_NULL, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
};

use super::images;
use super::items::{item, to_core};
use super::keys;

const ID_FIRST: u32 = 1;
const ID_LAST: u32 = 0x6FFF;
/// Eigener Eintrag „In neuem Tab öffnen“, außerhalb des Bereichs der Shell.
const ID_NEW_TAB: u32 = 0x7000;
/// Erlaubter Bereich für Einträge, die das Frontend ergänzt.
pub const EXTRA_IDS: std::ops::RangeInclusive<u32> = 0x7001..=0x70FF;
const WM_REQUEST: u32 = WM_APP + 1;

/// Verben, die die Oberfläche selbst ausführt.
const APP_VERBS: &[&str] = &["open", "cut", "copy", "paste", "delete", "rename"];

pub enum Target {
    /// Markierte Elemente (alle im selben Ordner).
    Items(Vec<ValidPath>),
    /// Leere Fläche eines Ordners.
    Background(ValidPath),
}

/// Zusätzlicher Eintrag der App im nativen Menü.
pub struct Extra {
    pub id: u32,
    pub label: String,
    /// Wird weggelassen, wenn das native Menü schon einen Eintrag mit einem
    /// dieser Texte (oder dem eigenen) hat, z. B. von OneDrive selbst.
    pub skip_if: Vec<String>,
}

/// Ein Eintrag des Menüs, wie ihn die Oberfläche zeichnet.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuItem {
    /// Position in der Menüebene von Windows (für Untermenüs maßgeblich).
    pub position: u32,
    pub id: u32,
    pub separator: bool,
    pub label: String,
    /// Tastenkürzel-Text (hinter dem Tabulator), z. B. „Strg+C“.
    pub shortcut: Option<String>,
    pub disabled: bool,
    pub checked: bool,
    /// Standardbefehl (fett).
    pub default: bool,
    pub submenu: bool,
    /// Symbol als Base64-kodiertes PNG.
    pub icon: Option<String>,
}

pub enum Command {
    Open {
        target: Target,
        /// Hauptfenster; erhält danach wieder den Vordergrund.
        owner: isize,
        /// Beschriftung für „In neuem Tab öffnen“; `None` lässt den Eintrag weg.
        new_tab_label: Option<String>,
        extras: Vec<Extra>,
    },
    /// Untermenü unter dem Positionspfad ab der obersten Ebene.
    Submenu {
        session: u32,
        path: Vec<u32>,
    },
    Invoke {
        session: u32,
        id: u32,
    },
    /// Zeigt das native Windows-Menü derselben Sitzung.
    Native {
        session: u32,
    },
    Close {
        session: u32,
    },
}

pub enum Reply {
    Menu { session: u32, items: Vec<MenuItem> },
    Items(Vec<MenuItem>),
    Outcome(Outcome),
    Closed,
}

struct Request {
    command: Command,
    reply: Sender<Result<Reply, CoreError>>,
}

#[derive(Debug)]
pub enum Outcome {
    Dismissed,
    /// Die Shell hat den Befehl selbst ausgeführt.
    Done,
    /// Die Oberfläche soll das Verb ausführen.
    Verb(String),
    NewTab,
    /// Ein ergänzter Eintrag der App wurde gewählt.
    Extra(u32),
}

type Queue = Arc<Mutex<VecDeque<Request>>>;

struct Host {
    hwnd: isize,
    queue: Queue,
}

enum Handler {
    V2(IContextMenu2),
    V3(IContextMenu3),
}

/// Das aktuell offene Menü (höchstens eines), samt allem, was es braucht.
struct Session {
    id: u32,
    menu: IContextMenu,
    hmenu: MenuGuard,
    extras: Vec<u32>,
    has_new_tab: bool,
    owner: isize,
}

thread_local! {
    static QUEUE: RefCell<Option<Queue>> = const { RefCell::new(None) };
    static CURRENT: RefCell<Option<Handler>> = const { RefCell::new(None) };
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    static NEXT_SESSION: Cell<u32> = const { Cell::new(1) };
}

fn host() -> Option<&'static Host> {
    static HOST: OnceLock<Option<Host>> = OnceLock::new();
    HOST.get_or_init(|| {
        let queue: Queue = Arc::default();
        let (ready_tx, ready_rx) = channel::<Option<isize>>();
        let thread_queue = Arc::clone(&queue);
        std::thread::Builder::new()
            .name("context-menu".into())
            .spawn(move || thread_main(&ready_tx, thread_queue))
            .ok()?;
        let hwnd = ready_rx.recv().ok().flatten()?;
        Some(Host { hwnd, queue })
    })
    .as_ref()
}

fn thread_main(ready: &Sender<Option<isize>>, queue: Queue) {
    let _com = super::com::ComGuard::sta();
    QUEUE.with(|q| *q.borrow_mut() = Some(queue));

    // SAFETY: Fensterklasse und verstecktes Fenster gehören diesem Thread für
    // die gesamte Prozesslaufzeit; die Zeichenketten sind statisch.
    let hwnd = unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: w!("ExplorerContextMenuHost"),
            ..Default::default()
        };
        RegisterClassW(&class);
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("ExplorerContextMenuHost"),
            w!(""),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
    };
    let Ok(hwnd) = hwnd else {
        let _ = ready.send(None);
        return;
    };
    let _ = ready.send(Some(hwnd.0 as isize));

    let mut msg = MSG::default();
    // SAFETY: Standard-Nachrichtenschleife des Threads, der das Fenster besitzt.
    unsafe {
        while GetMessageW(&raw mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);
        }
    }
}

/// Fensterprozedur des versteckten Fensters.
///
/// # Safety
/// Wird ausschließlich von Windows mit gültigen Argumenten aufgerufen.
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_REQUEST => {
            let request = QUEUE.with(|q| {
                q.borrow()
                    .as_ref()
                    .and_then(|q| q.lock().unwrap_or_else(PoisonError::into_inner).pop_front())
            });
            if let Some(request) = request {
                let reply = request.reply.clone();
                let _ = reply.send(process(hwnd, request.command));
            }
            LRESULT(0)
        }
        WM_INITMENUPOPUP | WM_MEASUREITEM | WM_DRAWITEM | WM_MENUCHAR => {
            match forward(msg, wparam, lparam) {
                Some(result) => result,
                // SAFETY: Argumente stammen unverändert von Windows.
                None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
            }
        }
        // SAFETY: Argumente stammen unverändert von Windows.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Reicht Menü-Nachrichten an die Shell-Erweiterung weiter (Untermenüs,
/// Owner-Draw-Einträge wie „Senden an“).
fn forward(msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    CURRENT.with(|current| {
        let current = current.borrow();
        let handler = current.as_ref()?;
        let mut result = LRESULT(0);
        // SAFETY: Das Menü-Objekt lebt, solange CURRENT gesetzt ist (siehe `process`).
        let outcome = unsafe {
            match handler {
                Handler::V3(menu) => {
                    menu.HandleMenuMsg2(msg, wparam, lparam, Some(&raw mut result))
                }
                Handler::V2(menu) => menu.HandleMenuMsg(msg, wparam, lparam),
            }
        };
        outcome.is_ok().then_some(result)
    })
}

/// Räumt das Menü beim Drop weg.
struct MenuGuard(HMENU);

impl Drop for MenuGuard {
    fn drop(&mut self) {
        // SAFETY: Das Menü gehört uns und wird genau einmal zerstört.
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

fn context_menu(target: &Target, hwnd: HWND) -> Result<IContextMenu, CoreError> {
    match target {
        Target::Items(paths) => {
            let array = super::items::array(paths)?;
            // SAFETY: gültiges Array; BHID_SFUIObject liefert das Kontextmenü der Elemente.
            unsafe { array.BindToHandler::<_, IContextMenu>(None, &BHID_SFUIObject) }
                .map_err(|e| to_core(&e))
        }
        Target::Background(dir) => {
            let folder_item = item(dir).map_err(|e| to_core(&e))?;
            // SAFETY: gültiges Item; BHID_SFObject liefert den Ordner als IShellFolder.
            let folder =
                unsafe { folder_item.BindToHandler::<_, IShellFolder>(None, &BHID_SFObject) }
                    .map_err(|e| to_core(&e))?;
            // SAFETY: gültiger Ordner; das Fenster ist unser verstecktes Hauptfenster dieses Threads.
            unsafe { folder.CreateViewObject::<IContextMenu>(hwnd) }.map_err(|e| to_core(&e))
        }
    }
}

fn verb_of(menu: &IContextMenu, offset: u32) -> Option<String> {
    let mut buffer = [0u16; 64];
    // SAFETY: Der Puffer fasst 64 UTF-16-Zeichen; die Größe wird mitgegeben.
    unsafe {
        menu.GetCommandString(
            offset as usize,
            GCS_VERBW,
            None,
            PSTR(buffer.as_mut_ptr().cast()),
            buffer.len() as u32,
        )
    }
    .ok()?;
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]).to_lowercase())
}

fn invoke(menu: &IContextMenu, hwnd: HWND, offset: u32, at: POINT) -> Result<(), CoreError> {
    let info = CMINVOKECOMMANDINFOEX {
        cbSize: size_of::<CMINVOKECOMMANDINFOEX>() as u32,
        fMask: SEE_MASK_UNICODE | CMIC_MASK_PTINVOKE,
        hwnd,
        // Der Befehl wird als Offset (MAKEINTRESOURCE) übergeben.
        lpVerb: PCSTR(offset as usize as *const u8),
        lpVerbW: PCWSTR(offset as usize as *const u16),
        nShow: SW_SHOWNORMAL.0,
        ptInvoke: at,
        ..Default::default()
    };
    // SAFETY: Die Struktur ist vollständig initialisiert und größer als CMINVOKECOMMANDINFO,
    // wie es die Schnittstelle für die Ex-Variante vorsieht.
    unsafe { menu.InvokeCommand((&raw const info).cast::<CMINVOKECOMMANDINFO>()) }
        .map_err(|e| to_core(&e))
}

/// Beschriftung ohne Tastenkürzel-Zusatz, für den Textvergleich.
fn normalize(text: &str) -> String {
    text.split('\t')
        .next()
        .unwrap_or_default()
        .replace('&', "")
        .trim()
        .to_lowercase()
}

/// Texte der Einträge der obersten Menüebene.
fn menu_texts(hmenu: HMENU) -> Vec<String> {
    // SAFETY: gültiges Menü; der Puffer fasst die übergebene Länge.
    unsafe {
        (0..GetMenuItemCount(Some(hmenu)).max(0))
            .map(|position| {
                let mut buffer = [0u16; 256];
                let len = GetMenuStringW(hmenu, position as u32, Some(&mut buffer), MF_BYPOSITION);
                normalize(&String::from_utf16_lossy(&buffer[..len.max(0) as usize]))
            })
            .collect()
    }
}

/// Hält Position und Schalter eines Menüeintrags fest.
const MAX_ITEMS_PER_MENU: i32 = 300;
const MAX_DEPTH: usize = 4;

fn clean_label(raw: &str) -> (String, Option<String>) {
    let mut parts = raw.splitn(2, '\t');
    let text = parts.next().unwrap_or_default();
    let shortcut = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    // „&“ markiert den Zugriffsbuchstaben, „&&“ ist ein echtes „&“.
    let mut label = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                label.push('&');
                chars.next();
            }
        } else {
            label.push(c);
        }
    }
    (label.trim().to_owned(), shortcut)
}

/// Liest die Einträge einer Menüebene aus.
fn read_items(hmenu: HMENU) -> Vec<MenuItem> {
    let mut items = Vec::new();
    // SAFETY: gültiges Menü auf dem Menü-Thread; alle Puffer sind lokal und ausreichend groß.
    unsafe {
        let count = GetMenuItemCount(Some(hmenu)).clamp(0, MAX_ITEMS_PER_MENU);
        for position in 0..count as u32 {
            let mut info = MENUITEMINFOW {
                cbSize: size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_ID | MIIM_FTYPE | MIIM_STATE | MIIM_SUBMENU | MIIM_BITMAP,
                ..Default::default()
            };
            if GetMenuItemInfoW(hmenu, position, true, &raw mut info).is_err() {
                continue;
            }
            if info.fType.0 & MFT_SEPARATOR.0 != 0 {
                items.push(MenuItem {
                    position,
                    id: 0,
                    separator: true,
                    label: String::new(),
                    shortcut: None,
                    disabled: true,
                    checked: false,
                    default: false,
                    submenu: false,
                    icon: None,
                });
                continue;
            }
            let mut buffer = [0u16; 256];
            let len = GetMenuStringW(hmenu, position, Some(&mut buffer), MF_BYPOSITION);
            let (label, shortcut) =
                clean_label(&String::from_utf16_lossy(&buffer[..len.max(0) as usize]));
            // Einträge, die sich nur selbst zeichnen und keinen Text liefern, lassen sich nicht darstellen.
            if label.is_empty() {
                continue;
            }
            // Pseudo-Handles (HBMMENU_*, Rückruf) sind keine Bitmaps.
            let icon = (info.hbmpItem.0 as isize > 11)
                .then(|| images::menu_icon_png(info.hbmpItem))
                .flatten()
                .map(|png| to_base64(&png));
            items.push(MenuItem {
                position,
                id: info.wID,
                separator: false,
                label,
                shortcut,
                disabled: info.fState.0 & (MFS_DISABLED.0 | MFS_GRAYED.0) != 0,
                checked: info.fState.0 & MFS_CHECKED.0 != 0,
                default: info.fState.0 & MFS_DEFAULT.0 != 0,
                submenu: !info.hSubMenu.is_invalid(),
                icon,
            });
        }
    }
    items
}

/// Gibt es in diesem Menü (auch in Untermenüs) einen Befehl mit dieser Kennung?
fn has_command(hmenu: HMENU, id: u32, depth: usize) -> bool {
    // SAFETY: gültiges Menü auf dem Menü-Thread.
    unsafe {
        for position in 0..GetMenuItemCount(Some(hmenu)).clamp(0, MAX_ITEMS_PER_MENU) as u32 {
            let mut info = MENUITEMINFOW {
                cbSize: size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_ID | MIIM_SUBMENU,
                ..Default::default()
            };
            if GetMenuItemInfoW(hmenu, position, true, &raw mut info).is_err() {
                continue;
            }
            if info.hSubMenu.is_invalid() {
                if info.wID == id {
                    return true;
                }
            } else if depth < MAX_DEPTH && has_command(info.hSubMenu, id, depth + 1) {
                return true;
            }
        }
    }
    false
}

fn close_session() {
    CURRENT.with(|c| *c.borrow_mut() = None);
    SESSION.with(|s| *s.borrow_mut() = None);
}

/// Alles, was zum Ausführen gebraucht wird, ohne den Zugriff auf `SESSION`
/// zu halten (die Shell darf dabei Nachrichten verarbeiten).
struct Parts {
    menu: IContextMenu,
    hmenu: HMENU,
    extras: Vec<u32>,
    has_new_tab: bool,
    owner: isize,
}

fn parts(session: u32) -> Result<Parts, CoreError> {
    SESSION.with(|s| {
        s.borrow()
            .as_ref()
            .filter(|s| s.id == session)
            .map(|s| Parts {
                menu: s.menu.clone(),
                hmenu: s.hmenu.0,
                extras: s.extras.clone(),
                has_new_tab: s.has_new_tab,
                owner: s.owner,
            })
            .ok_or(CoreError::NotFound)
    })
}

fn open(
    hwnd: HWND,
    target: &Target,
    owner: isize,
    new_tab_label: Option<&str>,
    extras: &[Extra],
) -> Result<Reply, CoreError> {
    close_session();
    let menu = context_menu(target, hwnd)?;
    let handler = menu
        .cast::<IContextMenu3>()
        .map(Handler::V3)
        .or_else(|_| menu.cast::<IContextMenu2>().map(Handler::V2))
        .ok();

    // SAFETY: Das Menü wird vom Guard zerstört; alle Handles bleiben auf diesem Thread.
    unsafe {
        let hmenu = CreatePopupMenu().map_err(|e| to_core(&e))?;
        let guard = MenuGuard(hmenu);

        let mut flags = CMF_NORMAL | CMF_CANRENAME;
        if keys::modifiers().shift {
            flags |= CMF_EXTENDEDVERBS;
        }
        menu.QueryContextMenu(hmenu, 0, ID_FIRST, ID_LAST, flags)
            .ok()
            .map_err(|e| to_core(&e))?;

        if let Some(label) = new_tab_label {
            let text = HSTRING::from(label);
            let _ = InsertMenuW(
                hmenu,
                1,
                MF_BYPOSITION | MF_STRING,
                ID_NEW_TAB as usize,
                PCWSTR(text.as_ptr()),
            );
        }

        // Einträge der App direkt hinter „Öffnen“ (und „In neuem Tab öffnen“).
        let existing = menu_texts(hmenu);
        let mut position = if new_tab_label.is_some() { 2 } else { 1 };
        let mut added = Vec::new();
        for extra in extras {
            let known = std::iter::once(&extra.label)
                .chain(&extra.skip_if)
                .any(|text| existing.contains(&normalize(text)));
            if known {
                continue;
            }
            let text = HSTRING::from(extra.label.as_str());
            let _ = InsertMenuW(
                hmenu,
                position,
                MF_BYPOSITION | MF_STRING,
                extra.id as usize,
                PCWSTR(text.as_ptr()),
            );
            added.push(extra.id);
            position += 1;
        }

        let items = read_items(hmenu);
        let id = NEXT_SESSION.with(|n| {
            let id = n.get();
            n.set(id.wrapping_add(1).max(1));
            id
        });
        CURRENT.with(|c| *c.borrow_mut() = handler);
        SESSION.with(|s| {
            *s.borrow_mut() = Some(Session {
                id,
                menu,
                hmenu: guard,
                extras: added,
                has_new_tab: new_tab_label.is_some(),
                owner,
            });
        });
        Ok(Reply::Menu { session: id, items })
    }
}

/// Befüllt (lazy, wie Windows beim Aufklappen) und liest ein Untermenü.
fn submenu(session: u32, path: &[u32]) -> Result<Vec<MenuItem>, CoreError> {
    if path.is_empty() || path.len() > MAX_DEPTH {
        return Err(CoreError::Unsupported);
    }
    let mut hmenu = parts(session)?.hmenu;
    for &position in path {
        // SAFETY: gültiges Menü auf dem Menü-Thread; ungültige Positionen liefern ein leeres Handle.
        let sub = unsafe { GetSubMenu(hmenu, position as i32) };
        if sub.is_invalid() {
            return Err(CoreError::NotFound);
        }
        // Wie beim Aufklappen: die Erweiterung füllt ihr Untermenü jetzt.
        let _ = forward(
            WM_INITMENUPOPUP,
            WPARAM(sub.0 as usize),
            LPARAM(position as isize & 0xFFFF),
        );
        hmenu = sub;
    }
    Ok(read_items(hmenu))
}

/// Führt den gewählten Befehl aus und beendet die Sitzung.
fn run_command(hwnd: HWND, parts: &Parts, id: u32, at: POINT) -> Result<Outcome, CoreError> {
    if id == ID_NEW_TAB && parts.has_new_tab {
        return Ok(Outcome::NewTab);
    }
    if parts.extras.contains(&id) {
        return Ok(Outcome::Extra(id));
    }
    // Nur Befehle, die das Menü wirklich enthält.
    if !(ID_FIRST..=ID_LAST).contains(&id) || !has_command(parts.hmenu, id, 0) {
        return Err(CoreError::Unsupported);
    }
    let offset = id - ID_FIRST;
    if let Some(verb) = verb_of(&parts.menu, offset) {
        if APP_VERBS.contains(&verb.as_str()) {
            return Ok(Outcome::Verb(verb));
        }
    }
    invoke(&parts.menu, hwnd, offset, at)?;
    Ok(Outcome::Done)
}

fn cursor() -> POINT {
    let mut at = POINT::default();
    // SAFETY: gültiger Ausgabezeiger.
    unsafe {
        let _ = GetCursorPos(&raw mut at);
    }
    at
}

fn restore_foreground(owner: isize) {
    if owner != 0 {
        // SAFETY: Fensterhandle des Hauptfensters; ein ungültiges Handle ist folgenlos.
        unsafe {
            let _ = SetForegroundWindow(HWND(owner as *mut _));
        }
    }
}

/// Rückfall: das Windows-Menü selbst anzeigen und die Wahl auswerten.
fn native(hwnd: HWND, session: u32) -> Result<Outcome, CoreError> {
    let parts = parts(session)?;
    let at = cursor();
    // SAFETY: Menü und Fenster gehören diesem Thread.
    let command = unsafe {
        // Ohne Vordergrund schließt das Menü beim Wegklicken nicht.
        let _ = SetForegroundWindow(hwnd);
        let command = TrackPopupMenuEx(
            parts.hmenu,
            (TPM_RETURNCMD | TPM_RIGHTBUTTON).0,
            at.x,
            at.y,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        command
    };
    restore_foreground(parts.owner);
    let id = command.0 as u32;
    if id == 0 {
        return Ok(Outcome::Dismissed);
    }
    run_command(hwnd, &parts, id, at)
}

fn process(hwnd: HWND, command: Command) -> Result<Reply, CoreError> {
    match command {
        Command::Open {
            target,
            owner,
            new_tab_label,
            extras,
        } => open(hwnd, &target, owner, new_tab_label.as_deref(), &extras),
        Command::Submenu { session, path } => submenu(session, &path).map(Reply::Items),
        Command::Invoke { session, id } => {
            let parts = parts(session)?;
            let result = run_command(hwnd, &parts, id, cursor());
            restore_foreground(parts.owner);
            close_session();
            result.map(Reply::Outcome)
        }
        Command::Native { session } => {
            let result = native(hwnd, session);
            close_session();
            result.map(Reply::Outcome)
        }
        Command::Close { session } => {
            if SESSION.with(|s| s.borrow().as_ref().is_some_and(|s| s.id == session)) {
                close_session();
            }
            Ok(Reply::Closed)
        }
    }
}

/// Schickt einen Befehl an den Menü-Thread und wartet auf die Antwort.
/// Blockiert den aufrufenden Worker-Thread, nicht die Oberfläche.
pub fn send(command: Command) -> Result<Reply, CoreError> {
    let host = host().ok_or(CoreError::Unsupported)?;
    let (reply, wait) = channel();
    host.queue
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push_back(Request { command, reply });
    // SAFETY: Das Fenster lebt für die Prozesslaufzeit; PostMessage ist thread-sicher.
    unsafe {
        PostMessageW(
            Some(HWND(host.hwnd as *mut _)),
            WM_REQUEST,
            WPARAM(0),
            LPARAM(0),
        )
    }
    .map_err(|_| CoreError::Unsupported)?;
    wait.recv().map_err(|_| CoreError::Unsupported)?
}
