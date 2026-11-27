//! What system-info's Plugin and its clients say to each other.
//!
//! The Core carries these bytes and never reads them, so their shape is agreed here, between the two
//! ends, and nowhere in Yoke. They are JSON, so that `yoke unit ask` prints something a person reads.

use serde::{Deserialize, Serialize};

/// The question a client asks for a [`Snapshot`].
pub const INFO_QUERY: &str = "system.info";

/// The stream that carries an [`Uptime`] every second while it flows.
pub const UPTIME_STREAM: &str = "system.uptime";

/// The version of the payloads below. A change a reader of the previous one cannot read raises it.
pub const SCHEMA: u32 = 1;

/// The host as the Plugin reads it: identification, which does not change while it runs, and the uptime
/// at the moment of the answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema: u32,
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub cpu_brand: String,
    pub physical_cores: u32,
    pub logical_cpus: u32,
    pub total_memory_bytes: u64,
    pub uptime_seconds: u64,
    /// What the operating system says of itself in `os-release`, each field absent where the file has
    /// none. Added within schema 1: a reader that does not know it skips it, and one reading an answer
    /// from before it reads it empty.
    #[serde(default)]
    pub os_release: OsRelease,
}

/// The fields of `os-release` a client shows, named as the file names them in lower case.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsRelease {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pretty_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bug_report_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_policy_url: Option<String>,
}

impl OsRelease {
    /// The fields read from the text of an `os-release` file: `KEY=value` lines, the value bare or quoted
    /// with `"` or `'`, a backslash in a double-quoted value escaping the character after it; comments,
    /// blank lines, other keys and empty values left out.
    pub fn parse(text: &str) -> OsRelease {
        let mut o = OsRelease::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, raw)) = line.split_once('=') else {
                continue;
            };
            let value = unquote(raw.trim());
            if value.is_empty() {
                continue;
            }
            let slot = match key.trim() {
                "ID" => &mut o.id,
                "NAME" => &mut o.name,
                "PRETTY_NAME" => &mut o.pretty_name,
                "LOGO" => &mut o.logo,
                "HOME_URL" => &mut o.home_url,
                "DOCUMENTATION_URL" => &mut o.documentation_url,
                "SUPPORT_URL" => &mut o.support_url,
                "BUG_REPORT_URL" => &mut o.bug_report_url,
                "PRIVACY_POLICY_URL" => &mut o.privacy_policy_url,
                _ => continue,
            };
            *slot = Some(value);
        }
        o
    }
}

fn unquote(raw: &str) -> String {
    let mut chars = raw.chars();
    match chars.next() {
        Some('\'') => raw[1..].strip_suffix('\'').unwrap_or(&raw[1..]).to_string(),
        Some('"') => {
            let inner = raw[1..].strip_suffix('"').unwrap_or(&raw[1..]);
            let mut out = String::new();
            let mut escaped = false;
            for c in inner.chars() {
                if escaped {
                    out.push(c);
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else {
                    out.push(c);
                }
            }
            out
        }
        _ => raw.to_string(),
    }
}

/// One frame of the uptime stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Uptime {
    pub uptime_seconds: u64,
}

impl Snapshot {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a snapshot is always written as JSON")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let s: Snapshot =
            serde_json::from_slice(bytes).map_err(|e| format!("not a snapshot: {e}"))?;
        if s.schema != SCHEMA {
            return Err(format!(
                "a snapshot of schema {}, and this reader reads {SCHEMA}",
                s.schema
            ));
        }
        Ok(s)
    }

    /// The total memory in MiB, as a panel shows it.
    pub fn total_memory_mib(&self) -> u64 {
        self.total_memory_bytes / (1024 * 1024)
    }
}

impl Uptime {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("an uptime is always written as JSON")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("not an uptime: {e}"))
    }
}

/// An uptime as a person reads it: `3d 04h 05m 06s`, or `04h 05m 06s` under a day.
pub fn format_uptime(secs: u64) -> String {
    let (d, h, m, s) = (secs / 86_400, secs / 3600 % 24, secs / 60 % 60, secs % 60);
    if d > 0 {
        format!("{d}d {h:02}h {m:02}m {s:02}s")
    } else {
        format!("{h:02}h {m:02}m {s:02}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        Snapshot {
            schema: SCHEMA,
            hostname: "bench".into(),
            os_name: "Arch Linux".into(),
            os_version: "rolling".into(),
            kernel_version: "6.18.57-1-lts".into(),
            cpu_brand: "AMD Ryzen".into(),
            physical_cores: 8,
            logical_cpus: 16,
            total_memory_bytes: 32 * 1024 * 1024 * 1024,
            uptime_seconds: 42,
            os_release: OsRelease {
                id: Some("arch".into()),
                pretty_name: Some("Arch Linux".into()),
                ..OsRelease::default()
            },
        }
    }

    #[test]
    fn a_snapshot_reads_back_as_it_was_written() {
        let s = snapshot();
        assert_eq!(Snapshot::from_bytes(&s.to_bytes()), Ok(s));
    }

    #[test]
    fn a_snapshot_of_another_schema_is_refused() {
        let mut s = snapshot();
        s.schema = SCHEMA + 1;
        assert!(Snapshot::from_bytes(&s.to_bytes()).is_err());
    }

    #[test]
    fn os_release_is_read_as_the_file_writes_it() {
        let o = OsRelease::parse(
            "# a comment\nNAME=\"Arch Linux\"\nPRETTY_NAME='Arch Linux'\nID=arch\nBUILD_ID=rolling\n\n\
             HOME_URL=\"https://archlinux.org/\"\nLOGO=archlinux-logo\nSUPPORT_URL=\"\"\n\
             BUG_REPORT_URL=\"https://example.org/say \\\"hi\\\"\"\n",
        );
        assert_eq!(
            o,
            OsRelease {
                id: Some("arch".into()),
                name: Some("Arch Linux".into()),
                pretty_name: Some("Arch Linux".into()),
                logo: Some("archlinux-logo".into()),
                home_url: Some("https://archlinux.org/".into()),
                bug_report_url: Some("https://example.org/say \"hi\"".into()),
                ..OsRelease::default()
            }
        );
    }

    #[test]
    fn an_absent_field_is_not_written_and_an_answer_from_before_reads_empty() {
        let s = snapshot();
        let text = String::from_utf8(s.to_bytes()).unwrap();
        assert!(text.contains("\"pretty_name\":\"Arch Linux\"") && !text.contains("logo"));
        let before = r#"{"schema":1,"hostname":"bench","os_name":"Arch Linux","os_version":"rolling",
            "kernel_version":"6.18","cpu_brand":"AMD","physical_cores":8,"logical_cpus":16,
            "total_memory_bytes":1,"uptime_seconds":2}"#;
        assert_eq!(
            Snapshot::from_bytes(before.as_bytes()).unwrap().os_release,
            OsRelease::default()
        );
    }

    #[test]
    fn an_uptime_reads_back_as_it_was_written() {
        let u = Uptime { uptime_seconds: 7 };
        assert_eq!(Uptime::from_bytes(&u.to_bytes()), Ok(u));
    }

    #[test]
    fn an_uptime_is_formatted_with_days_only_past_one() {
        assert_eq!(format_uptime(3_661), "01h 01m 01s");
        assert_eq!(
            format_uptime(3 * 86_400 + 4 * 3600 + 5 * 60 + 6),
            "3d 04h 05m 06s"
        );
    }
}
