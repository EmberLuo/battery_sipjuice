//! 系统强调色读取。优先支持 GNOME/Ubuntu 暴露的 gsettings accent-color。

use serde::Serialize;
use std::process::Command;

#[derive(Clone, Debug, Serialize)]
pub struct SystemAccentColor {
    pub name: String,
    pub color: String,
    pub strong: String,
}

pub fn detect() -> Option<SystemAccentColor> {
    detect_gnome()
}

fn detect_gnome() -> Option<SystemAccentColor> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "accent-color"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    accent_from_name(value.trim().trim_matches('\''))
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
    use super::accent_from_name;

    #[test]
    fn maps_known_gnome_accent_names() {
        assert_eq!(accent_from_name("orange").unwrap().color, "#ff6900");
        assert_eq!(accent_from_name("brown").unwrap().strong, "#986a44");
        assert!(accent_from_name("unknown").is_none());
    }
}
