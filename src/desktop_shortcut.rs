/// Modifier flags for the global desktop shortcuts. The digit is always 1–9.
pub fn desktop_modifiers(value: &str) -> Result<u32, String> {
    if value.trim().is_empty() {
        return Ok(0);
    }
    value.split('+').try_fold(0, |flags, modifier| {
        let flag = match modifier.trim().to_ascii_lowercase().as_str() {
            "alt" => 1,
            "ctrl" | "control" => 2,
            "shift" => 4,
            "win" | "super" | "meta" => 8,
            _ => return Err("Use Alt, Ctrl, Shift or Win before the desktop number.".into()),
        };
        Ok(flags | flag)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_shortcuts_require_modifiers_and_allow_disabling() {
        assert_eq!(desktop_modifiers("Alt"), Ok(1));
        assert_eq!(desktop_modifiers(" shift + CONTROL "), Ok(6));
        assert_eq!(desktop_modifiers("Alt+Win"), Ok(9));
        assert_eq!(desktop_modifiers(""), Ok(0));
        assert!(desktop_modifiers("1").is_err());
        assert!(desktop_modifiers("Alt+1").is_err());
        assert!(desktop_modifiers("Alt+").is_err());
    }
}
