//! Yapfast's icon on its window. Linux desktops find it from the window's app ID,
//! which names the desktop entry in packaging/linux, which an AppImage installs
//! for itself (`crate::desktop_entry`); Windows shows the executable's icon
//! (build.rs) in Explorer, and the window loads the same one. On Windows the
//! taskbar icon also shows how many DMs and friend requests are waiting, as
//! Discord's does.

use slint::winit_030::winit::window::WindowAttributes;

/// The window's app ID on Linux, and the name of its desktop entry and icon.
pub const APP_ID: &str = "io.github.yapfast.Yapfast";

/// Gives each window Slint makes the icon.
pub fn attributes(attributes: WindowAttributes) -> WindowAttributes {
    platform::attributes(attributes)
}

/// Shows `count` on Yapfast's taskbar icon, as Discord does: a red circle with
/// the number in the icon's corner, and "9+" past nine. Zero puts the plain icon
/// back. Only Windows draws it; elsewhere this does nothing.
pub fn show_count(window: &slint::Window, count: u64) {
    platform::show_count(window, count);
}

/// One digit, or "+", as strokes in a box 6 wide and 10 tall.
#[cfg_attr(not(windows), allow(dead_code))]
fn glyph(character: char) -> &'static str {
    match character {
        '0' => "M0,0 H6 V10 H0 Z",
        '1' => "M1,2.4 L3.6,0 V10",
        '2' => "M0,0 H6 V5 H0 V10 H6",
        '3' => "M0,0 H6 V10 H0 M0,5 H6",
        '4' => "M0,0 V5 H6 M6,0 V10",
        '5' => "M6,0 H0 V5 H6 V10 H0",
        '6' => "M6,0 H0 V10 H6 V5 H0",
        '7' => "M0,0 H6 V10",
        '8' => "M0,0 H6 V10 H0 Z M0,5 H6",
        '9' => "M6,5 H0 V0 H6 V10 H0",
        _ => "M0,5 H6 M3,2 V8",
    }
}

/// The logo, `side` pixels square, with a bright red circle for `count` in its
/// bottom right corner.
#[cfg_attr(not(windows), allow(dead_code))]
fn badge_svg(side: u32, count: u64) -> String {
    let text = if count > 9 {
        "9+".to_string()
    } else {
        count.to_string()
    };
    let glyphs = text.chars().count();
    // The glyphs stand 9 apart; scale them to sit inside the circle.
    let width = 6.0 + 9.0 * (glyphs - 1) as f32;
    let scale = if glyphs > 1 { 1.7 } else { 2.0 };
    let paths: String = text
        .chars()
        .enumerate()
        .map(|(index, character)| {
            format!(
                r#"<path transform="translate({} 0)" d="{}" />"#,
                9 * index,
                glyph(character)
            )
        })
        .collect();
    let badge = format!(
        r##"<circle cx="71" cy="71" r="19" fill="#ff3b3b" /><g transform="translate(71 71) scale({scale}) translate({} -5)" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{paths}</g>"##,
        -width / 2.0
    );
    include_str!("../packaging/icon.svg")
        .replacen(
            r#"width="64" height="64""#,
            &format!(r#"width="{side}" height="{side}""#),
            1,
        )
        .replacen("</svg>", &format!("{badge}</svg>"), 1)
}

#[cfg(target_os = "linux")]
mod platform {
    use slint::winit_030::winit::window::{Icon, WindowAttributes};

    /// X11 shows the window's own icon where the desktop entry isn't
    /// installed; Wayland takes the icon only from the entry, and ignores it.
    pub fn attributes(attributes: WindowAttributes) -> WindowAttributes {
        let icon = slint::Image::load_from_svg_data(include_bytes!("../packaging/icon.svg"))
            .ok()
            .and_then(|image| image.to_rgba8())
            .and_then(|pixels| {
                Icon::from_rgba(pixels.as_bytes().to_vec(), pixels.width(), pixels.height()).ok()
            });
        attributes.with_window_icon(icon)
    }

    /// The count is drawn on Windows only.
    pub fn show_count(_window: &slint::Window, _count: u64) {}
}

#[cfg(windows)]
mod platform {
    use std::cell::Cell;

    use slint::winit_030::{
        WinitWindowAccessor,
        winit::{
            dpi::PhysicalSize,
            platform::windows::{IconExtWindows, WindowAttributesExtWindows, WindowExtWindows},
            window::{Icon, WindowAttributes},
        },
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXICON, SM_CXSMICON, SYSTEM_METRICS_INDEX,
    };

    /// The icon in packaging/windows/yapfast.rc.
    const ICON: u16 = 1;

    thread_local! {
        /// The count on the taskbar icon now, which a window made later starts with.
        static SHOWN: Cell<u64> = const { Cell::new(0) };
    }

    /// The side in pixels Windows draws an icon of this kind.
    fn pixels(metric: SYSTEM_METRICS_INDEX) -> u32 {
        // SAFETY: GetSystemMetrics only reads a setting.
        unsafe { GetSystemMetrics(metric) as u32 }
    }

    /// The executable's icon at `side` pixels.
    fn resource(side: u32) -> Option<Icon> {
        Icon::from_resource(ICON, Some(PhysicalSize::new(side, side))).ok()
    }

    /// The taskbar's large icon: the executable's, or the logo with `count` on it.
    fn taskbar(count: u64) -> Option<Icon> {
        let side = pixels(SM_CXICON);
        if count == 0 {
            return resource(side);
        }
        slint::Image::load_from_svg_data(super::badge_svg(side, count).as_bytes())
            .ok()
            .and_then(|image| image.to_rgba8())
            .and_then(|pixels| {
                Icon::from_rgba(pixels.as_bytes().to_vec(), pixels.width(), pixels.height()).ok()
            })
            .or_else(|| resource(side))
    }

    /// The title bar's small icon and the taskbar's large one, each loaded at
    /// the size Windows draws it rather than scaled from one picture.
    pub fn attributes(attributes: WindowAttributes) -> WindowAttributes {
        attributes
            .with_window_icon(resource(pixels(SM_CXSMICON)))
            .with_taskbar_icon(taskbar(SHOWN.get()))
    }

    pub fn show_count(window: &slint::Window, count: u64) {
        if SHOWN.replace(count) == count {
            return;
        }
        let icon = taskbar(count);
        window.with_winit_window(|winit| winit.set_taskbar_icon(icon));
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    pub fn attributes(
        attributes: slint::winit_030::winit::window::WindowAttributes,
    ) -> slint::winit_030::winit::window::WindowAttributes {
        attributes
    }

    pub fn show_count(_window: &slint::Window, _count: u64) {}
}

#[cfg(test)]
mod tests {
    use super::badge_svg;

    #[test]
    fn the_badge_is_a_red_circle_in_the_corner() {
        for count in [1, 9, 10, 250] {
            let svg = badge_svg(32, count);
            let pixels = slint::Image::load_from_svg_data(svg.as_bytes())
                .ok()
                .and_then(|image| image.to_rgba8())
                .expect("the badge is a picture");
            assert_eq!(pixels.width(), 32);
            // Inside the circle, clear of the digits.
            let pixel = pixels.as_slice()[24 * 32 + 28];
            assert!(pixel.r > 200 && pixel.g < 100 && pixel.b < 100, "{count}");
        }
    }
}
