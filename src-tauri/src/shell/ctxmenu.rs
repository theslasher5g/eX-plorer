//! Das native Windows-Kontextmenü (`IContextMenu`), damit Erweiterungen wie
//! 7-Zip, „Öffnen mit“, OneDrive oder Git erscheinen.
//!
//! Das Menü läuft auf einem eigenen, dauerhaften Thread mit verstecktem
//! Fenster und Nachrichtenschleife. So blockiert es nie die Oberfläche, die
//! Nachrichten für Untermenüs (`IContextMenu2/3`) landen am richtigen Ort,
//! und Dialoge der Shell (z. B. „Eigenschaften“) behalten ihre Schleife.
//!
//! Befehle, die die App selbst besser kann (Ausschneiden, Kopieren, Einfügen,
//! Löschen, Umbenennen, Öffnen), werden nicht von der Shell ausgeführt,
//! sondern als Verb an die Oberfläche zurückgegeben.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem::size_of;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use explorer_core::path::ValidPath;
use explorer_core::CoreError;
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
    GetMessageW, InsertMenuW, PostMessageW, RegisterClassW, SetForegroundWindow, TrackPopupMenuEx,
    TranslateMessage, HMENU, MF_BYPOSITION, MF_STRING, MSG, SW_SHOWNORMAL, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_APP, WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM, WM_MENUCHAR, WM_NULL,
    WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
};

use super::items::{item, to_core};
use super::keys;

const ID_FIRST: u32 = 1;
const ID_LAST: u32 = 0x6FFF;
/// Eigener Eintrag „In neuem Tab öffnen“, außerhalb des Bereichs der Shell.
const ID_NEW_TAB: u32 = 0x7000;
const WM_REQUEST: u32 = WM_APP + 1;

/// Verben, die die Oberfläche selbst ausführt.
const APP_VERBS: &[&str] = &["open", "cut", "copy", "paste", "delete", "rename"];

pub enum Target {
    /// Markierte Elemente (alle im selben Ordner).
    Items(Vec<ValidPath>),
    /// Leere Fläche eines Ordners.
    Background(ValidPath),
}

pub struct Request {
    pub target: Target,
    /// Hauptfenster; erhält danach wieder den Vordergrund.
    pub owner: isize,
    /// Beschriftung für „In neuem Tab öffnen“; `None` lässt den Eintrag weg.
    pub new_tab_label: Option<String>,
    reply: Sender<Result<Outcome, CoreError>>,
}

#[derive(Debug)]
pub enum Outcome {
    Dismissed,
    /// Die Shell hat den Befehl selbst ausgeführt.
    Done,
    /// Die Oberfläche soll das Verb ausführen.
    Verb(String),
    NewTab,
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

thread_local! {
    static QUEUE: RefCell<Option<Queue>> = const { RefCell::new(None) };
    static CURRENT: RefCell<Option<Handler>> = const { RefCell::new(None) };
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
                let _ = reply.send(process(hwnd, request));
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

fn process(hwnd: HWND, request: Request) -> Result<Outcome, CoreError> {
    let menu = context_menu(&request.target, hwnd)?;
    let handler = menu
        .cast::<IContextMenu3>()
        .map(Handler::V3)
        .or_else(|_| menu.cast::<IContextMenu2>().map(Handler::V2))
        .ok();

    // SAFETY: Das Menü wird vom Guard zerstört; alle Handles bleiben auf diesem Thread.
    unsafe {
        let hmenu = CreatePopupMenu().map_err(|e| to_core(&e))?;
        let _guard = MenuGuard(hmenu);

        let mut flags = CMF_NORMAL | CMF_CANRENAME;
        if keys::modifiers().shift {
            flags |= CMF_EXTENDEDVERBS;
        }
        menu.QueryContextMenu(hmenu, 0, ID_FIRST, ID_LAST, flags)
            .ok()
            .map_err(|e| to_core(&e))?;

        if let Some(label) = &request.new_tab_label {
            let text = HSTRING::from(label.as_str());
            let _ = InsertMenuW(
                hmenu,
                1,
                MF_BYPOSITION | MF_STRING,
                ID_NEW_TAB as usize,
                PCWSTR(text.as_ptr()),
            );
        }

        CURRENT.with(|c| *c.borrow_mut() = handler);
        let mut at = POINT::default();
        let _ = GetCursorPos(&raw mut at);
        // Ohne Vordergrund schließt das Menü beim Wegklicken nicht.
        let _ = SetForegroundWindow(hwnd);
        let command = TrackPopupMenuEx(
            hmenu,
            (TPM_RETURNCMD | TPM_RIGHTBUTTON).0,
            at.x,
            at.y,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        CURRENT.with(|c| *c.borrow_mut() = None);

        if request.owner != 0 {
            let _ = SetForegroundWindow(HWND(request.owner as *mut _));
        }

        let id = command.0 as u32;
        if id == 0 {
            return Ok(Outcome::Dismissed);
        }
        if id == ID_NEW_TAB {
            return Ok(Outcome::NewTab);
        }
        let offset = id - ID_FIRST;
        if let Some(verb) = verb_of(&menu, offset) {
            if APP_VERBS.contains(&verb.as_str()) {
                return Ok(Outcome::Verb(verb));
            }
        }
        invoke(&menu, hwnd, offset, at)?;
        Ok(Outcome::Done)
    }
}

/// Zeigt das Menü an der Mauszeigerposition und wartet auf die Auswahl.
/// Blockiert den aufrufenden Worker-Thread, nicht die Oberfläche.
pub fn show(
    target: Target,
    owner: isize,
    new_tab_label: Option<String>,
) -> Result<Outcome, CoreError> {
    let host = host().ok_or(CoreError::Unsupported)?;
    let (reply, wait) = channel();
    host.queue
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push_back(Request {
            target,
            owner,
            new_tab_label,
            reply,
        });
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
