//! A single taskbar item for the player's independently movable windows.
use tao::window::{Icon, WindowBuilder};

fn icon() -> Icon {
    Icon::from_rgba(
        include_bytes!("../assets/braveamp-64.rgba").to_vec(),
        64,
        64,
    )
    .expect("Bundled 64×64 icon has a valid RGBA buffer")
}

/// Pass `None` for the main player, or its HWND for each secondary panel.
/// Owned windows stay movable outside the player, follow it on minimize and
/// do not have their own Windows taskbar buttons.
#[cfg(target_os = "windows")]
pub fn configure(builder: WindowBuilder, owner: Option<isize>) -> WindowBuilder {
    use tao::platform::windows::WindowBuilderExtWindows;
    let image = icon();
    let builder = builder
        .with_window_icon(Some(image.clone()))
        .with_taskbar_icon(Some(image));
    if let Some(hwnd) = owner {
        builder.with_owner_window(hwnd).with_skip_taskbar(true)
    } else {
        builder
    }
}

#[cfg(not(target_os = "windows"))]
pub fn configure(builder: WindowBuilder, _owner: Option<isize>) -> WindowBuilder {
    builder.with_window_icon(Some(icon()))
}
