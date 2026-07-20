//! 系统强调色读取。优先查询 GTK 当前主题实际使用的选中色，
//! gsettings 的强调色名称与静态调色板仅用于无 GTK 上下文时兜底。

use serde::Serialize;
use std::process::Command;

#[derive(Clone, Debug, Serialize)]
pub struct SystemAccentColor {
    pub name: String,
    pub color: String,
    pub strong: String,
}

pub fn detect() -> Option<SystemAccentColor> {
    #[cfg(target_os = "linux")]
    if let Some(accent) = detect_gtk_theme() {
        return Some(accent);
    }
    detect_gnome()
}

#[cfg(target_os = "linux")]
fn detect_gtk_theme() -> Option<SystemAccentColor> {
    use gtk::prelude::*;

    // GTK 对象只能在初始化它的主线程访问；Tauri 的同步命令通常就在该线程执行。
    // 若运行环境不同则安静地退回 gsettings 名称映射，避免触发 gtk-rs 的线程断言。
    if !gtk::is_initialized_main_thread() {
        return None;
    }

    let context = gtk::StyleContext::new();
    let color = context
        .lookup_color("accent_bg_color")
        .or_else(|| context.lookup_color("theme_selected_bg_color"))?;
    let color_hex = rgb_hex(color.red(), color.green(), color.blue());
    let strong_hex = rgb_hex(
        color.red() * 0.86,
        color.green() * 0.86,
        color.blue() * 0.86,
    );
    Some(SystemAccentColor {
        name: "system".to_string(),
        color: color_hex,
        strong: strong_hex,
    })
}

fn detect_gnome() -> Option<SystemAccentColor> {
    accent_from_name(&gnome_accent_name()?)
}

fn gnome_accent_name() -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "accent-color"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    Some(value.trim().trim_matches('\'').to_string())
}

fn rgb_hex(red: f64, green: f64, blue: f64) -> String {
    let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(red),
        channel(green),
        channel(blue)
    )
}

fn accent_from_name(name: &str) -> Option<SystemAccentColor> {
    let (color, strong) = match name {
        "blue" => ("#3584e4", "#1c71d8"),
        "teal" => ("#2190a4", "#1a727f"),
        "green" => ("#3a944a", "#2d7339"),
        "yellow" => ("#c88800", "#9c6f00"),
        "orange" => ("#ff6900", "#e55e00"),
        "red" => ("#e01b24", "#c01c28"),
        "pink" => ("#d56199", "#b5487d"),
        "purple" => ("#9141ac", "#7d3e9d"),
        "slate" => ("#6f8396", "#5e7181"),
        "brown" => ("#b5835a", "#986a44"),
        _ => return None,
    };
    Some(SystemAccentColor {
        name: name.to_string(),
        color: color.to_string(),
        strong: strong.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{accent_from_name, rgb_hex};

    #[test]
    fn maps_known_gnome_accent_names() {
        assert_eq!(accent_from_name("orange").unwrap().color, "#ff6900");
        assert_eq!(accent_from_name("brown").unwrap().strong, "#986a44");
        assert!(accent_from_name("unknown").is_none());
    }

    #[test]
    fn formats_gtk_rgb_channels_as_css_hex() {
        assert_eq!(rgb_hex(0.454902, 0.376471, 0.831373), "#7460d4");
        assert_eq!(rgb_hex(-1.0, 0.5, 2.0), "#0080ff");
    }
}
