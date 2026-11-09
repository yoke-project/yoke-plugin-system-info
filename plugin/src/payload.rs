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
