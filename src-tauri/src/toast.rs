//! Windows notifications for updates, and links to the changelog.
//!
//! Toasts are shown under DIO's AppUserModelID, which the installer sets on the Start menu
//! shortcut, so they carry DIO's name and icon and follow the user's notification settings.

use crate::updates::Toast;

/// The changelog on DIO's website. Every release-notes link is built from this, never from input.
pub const CHANGELOG: &str = "https://nathandalpatan.github.io/dio/changelog/";

/// The changelog anchor for a version: `0.2.1` becomes `v0-2-1`. The website uses the same rule.
pub fn anchor(version: &semver::Version) -> String {
    let text: String = version
        .to_string()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("v{text}")
}

/// The changelog, or one version's entry when `version` is a valid version number.
pub fn release_notes_url(version: Option<&str>) -> Result<String, String> {
    match version {
        None => Ok(CHANGELOG.into()),
        Some(text) => semver::Version::parse(text.trim_start_matches('v'))
            .map(|version| format!("{CHANGELOG}#{}", anchor(&version)))
            .map_err(|_| "Invalid version.".to_owned()),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Arguments a toast passes back when the user clicks one of its buttons.
pub const VIEW_UPDATE: &str = "action=view";
pub const LATER: &str = "action=later";

pub fn xml(toast: &Toast) -> String {
    match toast {
        Toast::Available { version } => format!(
            concat!(
                r#"<toast launch="{view}" activationType="foreground"><visual><binding template="ToastGeneric">"#,
                r#"<text>An update is available</text><text>DIO v{version} is ready to download.</text>"#,
                r#"</binding></visual><actions>"#,
                r#"<action content="View update" arguments="{view}" activationType="foreground"/>"#,
                r#"<action content="Later" arguments="{later}" activationType="foreground"/>"#,
                r#"</actions></toast>"#
            ),
            view = VIEW_UPDATE,
            later = LATER,
            version = escape(version),
        ),
        Toast::Updated { version, summary } => {
            // The link opens only when the user clicks it, never on its own.
            let url = release_notes_url(Some(version)).unwrap_or_else(|_| CHANGELOG.into());
            let summary = if summary.is_empty() {
                String::new()
            } else {
                format!("<text>{}</text>", escape(summary))
            };
            format!(
                concat!(
                    r#"<toast><visual><binding template="ToastGeneric">"#,
                    r#"<text>DIO has been updated to v{version}</text>{summary}"#,
                    r#"</binding></visual><actions>"#,
                    r#"<action content="View what’s new" arguments="{url}" activationType="protocol"/>"#,
                    r#"</actions></toast>"#
                ),
                version = escape(version),
                summary = summary,
                url = escape(&url),
            )
        }
    }
}

/// Shows a toast. `on_click` receives the arguments of the button or body the user clicked.
/// Must run on a thread with COM initialised, such as the main thread.
pub fn show(
    app_id: &str,
    toast: &Toast,
    on_click: impl Fn(String) + Send + 'static,
) -> windows::core::Result<()> {
    use windows::core::{Interface, HSTRING};
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::Foundation::TypedEventHandler;
    use windows::UI::Notifications::{
        ToastActivatedEventArgs, ToastNotification, ToastNotificationManager,
    };
    let document = XmlDocument::new()?;
    document.LoadXml(&HSTRING::from(xml(toast)))?;
    let notification = ToastNotification::CreateToastNotification(&document)?;
    // Only foreground buttons arrive here; protocol buttons are opened by Windows itself.
    notification.Activated(&TypedEventHandler::new(
        move |_, args: windows::core::Ref<'_, windows::core::IInspectable>| {
            if let Some(arguments) = args
                .as_ref()
                .and_then(|args| args.cast::<ToastActivatedEventArgs>().ok())
                .and_then(|args| args.Arguments().ok())
            {
                on_click(arguments.to_string());
            }
            Ok(())
        },
    ))?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?.Show(&notification)
}

/// Opens an https address in the default browser.
pub fn open_url(url: &str) -> Result<(), String> {
    use windows::core::{w, HSTRING};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    if !url.starts_with("https://") {
        return Err("Only https links can be opened.".into());
    }
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(url),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err("Could not open your web browser.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_notes_links_point_at_the_version_anchor() {
        assert_eq!(release_notes_url(None).unwrap(), CHANGELOG);
        assert_eq!(
            release_notes_url(Some("0.2.1")).unwrap(),
            "https://nathandalpatan.github.io/dio/changelog/#v0-2-1"
        );
        assert_eq!(
            release_notes_url(Some("v1.0.0-rc.1")).unwrap(),
            "https://nathandalpatan.github.io/dio/changelog/#v1-0-0-rc-1"
        );
        for bad in ["", "latest", "1.0", "1.0.0\"><script>", "../../etc"] {
            assert!(release_notes_url(Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn toast_text_is_escaped() {
        let xml = xml(&Toast::Updated {
            version: "0.2.1".into(),
            summary: "Fixes <b> & \"quotes\"".into(),
        });
        assert!(xml.contains("Fixes &lt;b&gt; &amp; &quot;quotes&quot;"));
        assert!(xml.contains(r##"arguments="https://nathandalpatan.github.io/dio/changelog/#v0-2-1" activationType="protocol""##));
        assert!(
            !xml.contains("launch="),
            "clicking the body never opens the browser"
        );
    }

    #[test]
    fn available_toast_offers_view_and_later() {
        let xml = xml(&Toast::Available {
            version: "0.2.1".into(),
        });
        assert!(xml.contains("An update is available"));
        assert!(xml.contains("DIO v0.2.1 is ready to download."));
        assert!(xml.contains(r#"content="View update" arguments="action=view""#));
        assert!(xml.contains(r#"content="Later" arguments="action=later""#));
    }

    #[test]
    fn toast_xml_parses() {
        use windows::core::HSTRING;
        use windows::Data::Xml::Dom::XmlDocument;
        for toast in [
            Toast::Available {
                version: "0.2.1".into(),
            },
            Toast::Updated {
                version: "0.2.1".into(),
                summary: "A & B".into(),
            },
        ] {
            let document = XmlDocument::new().unwrap();
            document.LoadXml(&HSTRING::from(xml(&toast))).unwrap();
        }
    }

    #[test]
    fn only_https_links_open() {
        assert!(open_url("file:///C:/Windows/system32/calc.exe").is_err());
        assert!(open_url("javascript:alert(1)").is_err());
    }
}
