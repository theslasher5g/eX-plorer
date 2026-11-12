//! Startet einen echten OLE-Drag mit Dateien. So lassen sich Dateien aus der
//! App in andere Programme (Explorer, Mail, Browser-Upload …) ziehen, und
//! derselbe Weg dient auch für interne Drops auf Ordner, Tabs und Seitenleiste.

use explorer_core::path::ValidPath;
use explorer_core::CoreError;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::Ole::{DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    BHID_DataObject, ILFree, SHCreateShellItemArrayFromIDLists, SHDoDragDrop, SHParseDisplayName,
};

/// Gibt die Shell-IDLists beim Drop wieder frei.
struct IdLists(Vec<*mut ITEMIDLIST>);

impl Drop for IdLists {
    fn drop(&mut self) {
        for pidl in self.0.drain(..) {
            // SAFETY: Jeder Zeiger stammt aus SHParseDisplayName und wird genau einmal freigegeben.
            unsafe { ILFree(Some(pidl)) };
        }
    }
}

fn data_object(paths: &[ValidPath]) -> Result<IDataObject, CoreError> {
    let mut lists = IdLists(Vec::with_capacity(paths.len()));
    for path in paths {
        let wide = HSTRING::from(path.as_str());
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        // SAFETY: `wide` ist nullterminiert; `pidl` ist ein gültiger Ausgabezeiger.
        unsafe { SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &raw mut pidl, 0, None) }
            .map_err(|e| explorer_core::winerr::from_hresult(e.code().0))?;
        lists.0.push(pidl);
    }
    let raw: Vec<*const ITEMIDLIST> = lists.0.iter().map(|p| p.cast_const()).collect();
    // SAFETY: `raw` enthält gültige absolute IDLists, die bis zum Ende dieser Funktion leben.
    let array = unsafe { SHCreateShellItemArrayFromIDLists(&raw) }
        .map_err(|e| explorer_core::winerr::from_hresult(e.code().0))?;
    // SAFETY: gültiges Array; BHID_DataObject liefert ein IDataObject.
    unsafe { array.BindToHandler::<_, IDataObject>(None, &BHID_DataObject) }
        .map_err(|e| explorer_core::winerr::from_hresult(e.code().0))
}

/// Läuft modal und **nur auf dem UI-Thread** (dort ist die Maus erfasst).
/// Gibt den vom Ziel gewählten Drop-Effekt zurück (0 = abgebrochen).
pub fn run(paths: &[ValidPath]) -> Result<u32, CoreError> {
    let data = data_object(paths)?;
    // SAFETY: Das Datenobjekt ist gültig; ohne eigene Drop-Quelle nutzt die
    // Shell die Standardquelle samt Ziehbild.
    let effect = unsafe {
        SHDoDragDrop(
            None,
            &data,
            None,
            DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK,
        )
    };
    match effect {
        Ok(effect) => Ok(effect.0),
        // DRAGDROP_S_CANCEL ist ein Erfolgscode, kommt aber teils als Fehler an.
        Err(e) if e.code().0 == 0x0004_0101 => Ok(0),
        Err(e) => Err(explorer_core::winerr::from_hresult(e.code().0)),
    }
}
