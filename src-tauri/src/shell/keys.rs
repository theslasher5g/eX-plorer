//! Tastenstatus zum Zeitpunkt eines Drops.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_CONTROL, VK_SHIFT,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
}

pub fn modifiers() -> Modifiers {
    // SAFETY: GetAsyncKeyState hat keine Vorbedingungen; das höchste Bit zeigt „gedrückt“.
    let down = |vk: VIRTUAL_KEY| unsafe { GetAsyncKeyState(i32::from(vk.0)) } < 0;
    Modifiers {
        ctrl: down(VK_CONTROL),
        shift: down(VK_SHIFT),
    }
}
