#![allow(clippy::unnecessary_cast)]

use std::{
    io,
    time::{Duration, Instant},
};

use proc_maps::{MapRange, Pid};
use read_process_memory::{CopyAddress, ProcessHandle};
use snafu::{OptionExt, ResultExt, Snafu};
use sysinfo::{self, PidExt, ProcessExt};
use winapi::um::{
    handleapi::CloseHandle, processthreadsapi::OpenProcess, winnt::PROCESS_QUERY_INFORMATION,
    wow64apiset::IsWow64Process,
};

use crate::runtime::ProcessList;

#[derive(Debug, Snafu)]
#[snafu(context(suffix(false)))]
pub enum OpenError {
    ProcessDoesntExist,
    InvalidHandle { source: io::Error },
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(false)))]
pub enum ModuleError {
    ModuleDoesntExist,
    ListModules { source: io::Error },
}

pub type Address = u64;

pub struct Process {
    handle: ProcessHandle,
    pid: Pid,
    modules: Vec<MapRange>,
    last_check: Instant,
}

impl std::fmt::Debug for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Process").field("pid", &self.pid).finish()
    }
}

impl Process {
    pub fn with_name(name: &str, process_list: &mut ProcessList) -> Result<Self, OpenError> {
        process_list.refresh();
        let processes = process_list.processes_by_name(name);

        // Sorts the processes (asc) by numeric pid, to allow max_by_key to
        // select the higher pid in case all records are equally maximum; otherwise
        // use the process that was started the most recently, it's more
        // predictable for the user.

        let pid = processes
            .max_by_key(|p| (p.start_time(), p.pid().as_u32()))
            .context(ProcessDoesntExist)?
            .pid()
            .as_u32() as Pid;

        let handle = pid.try_into().context(InvalidHandle)?;

        Ok(Process {
            handle,
            pid,
            modules: Vec::new(),
            last_check: Instant::now() - Duration::from_secs(1),
        })
    }

    pub fn is_open(&self, process_list: &mut ProcessList) -> bool {
        // FIXME: We can actually ask the list to only refresh the individual process.
        process_list.refresh();
        process_list.is_open(sysinfo::Pid::from_u32(self.pid as u32))
    }

    pub fn module_address(&mut self, module: &str) -> Result<Address, ModuleError> {
        let now = Instant::now();
        if now - self.last_check >= Duration::from_secs(1) {
            self.modules = match proc_maps::get_process_maps(self.pid) {
                Ok(m) => m,
                Err(source) => {
                    self.modules.clear();
                    return Err(ModuleError::ListModules { source });
                }
            };
            self.last_check = now;
        }
        self.modules
            .iter()
            .find(|m| m.filename().map_or(false, |f| f.ends_with(module)))
            .context(ModuleDoesntExist)
            .map(|m| m.start() as u64)
    }

    pub fn module_size(&mut self, module: &str) -> Result<u64, ModuleError> {
        let now = Instant::now();
        if now - self.last_check >= Duration::from_secs(1) {
            self.modules = match proc_maps::get_process_maps(self.pid) {
                Ok(m) => m,
                Err(source) => {
                    self.modules.clear();
                    return Err(ModuleError::ListModules { source });
                }
            };
            self.last_check = now;
        }
        Ok(self
            .modules
            .iter()
            .filter(|m| m.filename().map_or(false, |f| f.ends_with(module)))
            .map(|m| m.size() as u64)
            .sum())
    }

    pub fn read_mem(&self, address: Address, buf: &mut [u8]) -> io::Result<()> {
        self.handle.copy_address(address as usize, buf)
    }

    pub fn bit_width(&self) -> u32 {
        #[cfg(all(windows, target_arch = "x86_64"))]
        {
            unsafe {
                let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, self.pid);
                if handle.is_null() {
                    return 0;
                }
                let mut is_32_bit = 0;
                let result = IsWow64Process(handle, &mut is_32_bit);
                let bits = if result != 0 {
                    if is_32_bit != 0 {
                        32
                    } else {
                        64
                    }
                } else {
                    0
                };
                CloseHandle(handle);
                bits
            }
        }
        #[cfg(not(all(windows, target_arch = "x86_64")))]
        {
            0
        }
    }
}
