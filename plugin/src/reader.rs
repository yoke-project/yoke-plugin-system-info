//! What the host is, read with `sysinfo`.

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use crate::payload::{SCHEMA, Snapshot};

/// The host's identification, which does not change while the Plugin runs, read once.
#[derive(Debug, Clone)]
pub struct Host {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub cpu_brand: String,
    pub physical_cores: u32,
    pub logical_cpus: u32,
    pub total_memory_bytes: u64,
}

impl Host {
    pub fn read() -> Host {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        sys.refresh_memory();
        Host {
            hostname: System::host_name().unwrap_or_default(),
            os_name: System::name().unwrap_or_default(),
            os_version: System::os_version().unwrap_or_default(),
            kernel_version: System::kernel_version().unwrap_or_default(),
            cpu_brand: sys
                .cpus()
                .first()
                .map(|c| c.brand().trim().to_string())
                .unwrap_or_default(),
            physical_cores: System::physical_core_count().unwrap_or(0) as u32,
            logical_cpus: sys.cpus().len() as u32,
            total_memory_bytes: sys.total_memory(),
        }
    }

    /// The snapshot this host answers with, at the uptime given.
    pub fn snapshot(&self, uptime_seconds: u64) -> Snapshot {
        Snapshot {
            schema: SCHEMA,
            hostname: self.hostname.clone(),
            os_name: self.os_name.clone(),
            os_version: self.os_version.clone(),
            kernel_version: self.kernel_version.clone(),
            cpu_brand: self.cpu_brand.clone(),
            physical_cores: self.physical_cores,
            logical_cpus: self.logical_cpus,
            total_memory_bytes: self.total_memory_bytes,
            uptime_seconds,
        }
    }
}

/// The uptime now, which changes every second and is read at every use.
pub fn uptime() -> u64 {
    System::uptime()
}
