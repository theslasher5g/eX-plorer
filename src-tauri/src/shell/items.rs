//! Gemeinsame Helfer für Shell-Items und -Arrays aus geprüften Pfaden.

use explorer_core::path::ValidPath;
use explorer_core::{winerr, CoreError};
use windows::core::{Error as WinError, HSTRING, PCWSTR};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    ILFree, IShellItem, IShellItemArray, SHCreateItemFromParsingName,
    SHCreateShellItemArrayFromIDLists, SHParseDisplayName,
};

pub fn to_core(err: &WinError) -> CoreError {
    winerr::from_hresult(err.code().0)
}

pub fn item(path: &ValidPath) -> Result<IShellItem, WinError> {
    let wide = HSTRING::from(path.as_str());
    // SAFETY: `wide` ist ein gültiger, nullterminierter UTF-16-String, der den Aufruf überlebt.
    unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) }
}

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

pub fn array(paths: &[ValidPath]) -> Result<IShellItemArray, CoreError> {
    let mut lists = IdLists(Vec::with_capacity(paths.len()));
    for path in paths {
        let wide = HSTRING::from(path.as_str());
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        // SAFETY: `wide` ist nullterminiert; `pidl` ist ein gültiger Ausgabezeiger.
        unsafe { SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &raw mut pidl, 0, None) }
            .map_err(|e| to_core(&e))?;
        lists.0.push(pidl);
    }
    let raw: Vec<*const ITEMIDLIST> = lists.0.iter().map(|p| p.cast_const()).collect();
    // SAFETY: `raw` enthält gültige absolute IDLists, die bis zum Ende dieser Funktion leben.
    unsafe { SHCreateShellItemArrayFromIDLists(&raw) }.map_err(|e| to_core(&e))
}
