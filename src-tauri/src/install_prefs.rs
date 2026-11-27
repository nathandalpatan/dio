//! The update preference shared with the installer through `HKCU\Software\<identifier>`.
//!
//! The installer writes `UpdateMode` and sets `InstallerChoice` to 1. DIO adopts that choice once at
//! startup, then mirrors later changes back so a reinstall preselects the current preference.

use crate::model::UPDATE_MODES;

const MODE: &str = "UpdateMode";
const INSTALLER_CHOICE: &str = "InstallerChoice";

pub trait Store {
    fn read(&self, name: &str) -> Option<String>;
    fn write(&self, name: &str, value: &str) -> Result<(), String>;
    fn remove(&self, name: &str);
}

/// What startup should do with the saved preference.
#[derive(Debug, PartialEq)]
pub struct Adoption {
    pub mode: String,
    /// The settings file must be saved with `mode`.
    pub changed: bool,
    /// The installer's choice was used; clear its flag once the settings are saved.
    pub from_installer: bool,
}

/// A choice just made in the installer wins, then the saved setting, then a value the installer wrote
/// earlier, then automatic updates.
pub fn adopt(saved: Option<&str>, store: &dyn Store) -> Adoption {
    let installed = store
        .read(MODE)
        .filter(|mode| UPDATE_MODES.contains(&mode.as_str()));
    let from_installer =
        store.read(INSTALLER_CHOICE).as_deref() == Some("1") && installed.is_some();
    let mode = if from_installer {
        installed.unwrap()
    } else {
        saved
            .map(str::to_owned)
            .or(installed)
            .unwrap_or_else(|| "auto".into())
    };
    Adoption {
        changed: saved != Some(mode.as_str()),
        mode,
        from_installer,
    }
}

/// Records the mode for the installer and clears the one-time installer flag.
pub fn remember(store: &dyn Store, mode: &str, clear_installer_choice: bool) -> Result<(), String> {
    if store.read(MODE).as_deref() != Some(mode) {
        store.write(MODE, mode)?;
    }
    if clear_installer_choice {
        store.remove(INSTALLER_CHOICE);
    }
    Ok(())
}

pub struct Registry {
    key: String,
}

impl Registry {
    pub fn new(identifier: &str) -> Self {
        Self {
            key: format!("Software\\{identifier}"),
        }
    }
}

impl Store for Registry {
    fn read(&self, name: &str) -> Option<String> {
        use windows::core::HSTRING;
        use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
        let mut buffer = [0u16; 64];
        let mut size = std::mem::size_of_val(&buffer) as u32;
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &HSTRING::from(&self.key),
                &HSTRING::from(name),
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if result.is_err() {
            return None;
        }
        let length = (size as usize / 2).saturating_sub(1).min(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..length]))
    }
    fn write(&self, name: &str, value: &str) -> Result<(), String> {
        use windows::core::HSTRING;
        use windows::Win32::System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};
        let data: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &HSTRING::from(&self.key),
                &HSTRING::from(name),
                REG_SZ.0,
                Some(data.as_ptr().cast()),
                (data.len() * 2) as u32,
            )
        }
        .ok()
        .map_err(|e| format!("Could not save the update preference for the installer: {e}"))
    }
    fn remove(&self, name: &str) {
        use windows::core::HSTRING;
        use windows::Win32::System::Registry::{RegDeleteKeyValueW, HKEY_CURRENT_USER};
        let _ = unsafe {
            RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                &HSTRING::from(&self.key),
                &HSTRING::from(name),
            )
        };
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    pub struct Memory(pub RefCell<HashMap<String, String>>);
    impl Store for Memory {
        fn read(&self, name: &str) -> Option<String> {
            self.0.borrow().get(name).cloned()
        }
        fn write(&self, name: &str, value: &str) -> Result<(), String> {
            self.0.borrow_mut().insert(name.into(), value.into());
            Ok(())
        }
        fn remove(&self, name: &str) {
            self.0.borrow_mut().remove(name);
        }
    }
    fn store(values: &[(&str, &str)]) -> Memory {
        let memory = Memory::default();
        for (name, value) in values {
            memory.write(name, value).unwrap();
        }
        memory
    }

    #[test]
    fn fresh_install_without_installer_value_defaults_to_automatic() {
        let adoption = adopt(None, &store(&[]));
        assert_eq!(
            (
                adoption.mode.as_str(),
                adoption.changed,
                adoption.from_installer
            ),
            ("auto", true, false)
        );
    }
    #[test]
    fn installer_choice_overrides_the_saved_setting_once() {
        let registry = store(&[(MODE, "manual"), (INSTALLER_CHOICE, "1")]);
        let adoption = adopt(Some("auto"), &registry);
        assert_eq!(
            (
                adoption.mode.as_str(),
                adoption.changed,
                adoption.from_installer
            ),
            ("manual", true, true)
        );
        remember(&registry, &adoption.mode, adoption.from_installer).unwrap();
        assert_eq!(registry.read(INSTALLER_CHOICE), None);
        // Next start: the saved setting is kept, even after the user changes it in Settings.
        let next = adopt(Some("notify"), &registry);
        assert_eq!((next.mode.as_str(), next.changed), ("notify", false));
    }
    #[test]
    fn saved_setting_wins_over_a_stale_installer_value() {
        let adoption = adopt(Some("notify"), &store(&[(MODE, "manual")]));
        assert_eq!(
            (adoption.mode.as_str(), adoption.changed),
            ("notify", false)
        );
    }
    #[test]
    fn earlier_installer_value_fills_a_missing_setting() {
        let adoption = adopt(None, &store(&[(MODE, "notify")]));
        assert_eq!((adoption.mode.as_str(), adoption.changed), ("notify", true));
    }
    #[test]
    fn invalid_registry_values_are_ignored() {
        let adoption = adopt(
            Some("manual"),
            &store(&[(MODE, "never"), (INSTALLER_CHOICE, "1")]),
        );
        assert_eq!(
            (adoption.mode.as_str(), adoption.from_installer),
            ("manual", false)
        );
    }
    #[test]
    fn registry_round_trip() {
        use windows::core::HSTRING;
        use windows::Win32::System::Registry::{RegDeleteKeyW, HKEY_CURRENT_USER};
        let identifier = format!("app.dio.dictionary.unit-test-{}", std::process::id());
        let registry = Registry::new(&identifier);
        assert_eq!(registry.read(MODE), None);
        registry.write(MODE, "notify").unwrap();
        assert_eq!(registry.read(MODE).as_deref(), Some("notify"));
        registry.remove(MODE);
        assert_eq!(registry.read(MODE), None);
        let _ = unsafe {
            RegDeleteKeyW(
                HKEY_CURRENT_USER,
                &HSTRING::from(format!("Software\\{identifier}")),
            )
        };
    }
    #[test]
    fn remember_mirrors_the_mode_for_the_next_install() {
        let registry = store(&[(MODE, "auto")]);
        remember(&registry, "manual", false).unwrap();
        assert_eq!(registry.read(MODE).as_deref(), Some("manual"));
    }
}
