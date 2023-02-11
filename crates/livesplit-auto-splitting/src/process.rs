#![allow(clippy::unnecessary_cast)]

use std::{
    io,
    path::Path,
    time::{Duration, Instant},
};

use proc_maps::{MapRange, Pid};
use read_process_memory::{CopyAddress, ProcessHandle};
use snafu::{OptionExt, ResultExt, Snafu};
use sysinfo::{self, PidExt, ProcessExt};

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
    path: Option<Box<str>>,
}

impl std::fmt::Debug for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Process")
            .field("pid", &self.pid)
            .field("path", &self.path)
            .finish()
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

        let process = &processes
            .max_by_key(|p| (p.start_time(), p.pid().as_u32()))
            .context(ProcessDoesntExist)?;

        let path = build_path(process.exe());

        let pid = process.pid().as_u32() as Pid;

        let handle = pid.try_into().context(InvalidHandle)?;

        Ok(Process {
            handle,
            pid,
            modules: Vec::new(),
            last_check: Instant::now() - Duration::from_secs(1),
            path,
        })
    }

    pub fn is_open(&self, process_list: &mut ProcessList) -> bool {
        // FIXME: We can actually ask the list to only refresh the individual process.
        process_list.refresh();
        process_list.is_open(sysinfo::Pid::from_u32(self.pid as u32))
    }

    pub fn module_address(&mut self, module: &str) -> Result<Address, ModuleError> {
        self.refresh_modules()?;
        self.modules
            .iter()
            .find(|m| m.filename().map_or(false, |f| f.ends_with(module)))
            .context(ModuleDoesntExist)
            .map(|m| m.start() as u64)
    }

    pub fn module_size(&mut self, module: &str) -> Result<u64, ModuleError> {
        self.refresh_modules()?;
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

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn module_path(&mut self, module: &str) -> Option<Box<str>> {
        self.refresh_modules().ok()?;
        let module = self
            .modules
            .iter()
            .find(|m| m.filename().map_or(false, |f| f.ends_with(module)))?;

        build_path(module.filename()?)
    }

    fn refresh_modules(&mut self) -> Result<(), ModuleError> {
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
        Ok(())
    }
}

fn build_path(original_path: &Path) -> Option<Box<str>> {
    let mut path = String::from("/mnt");
    for component in original_path.components() {
        if !path.ends_with('/') {
            path.push('/');
        }
        match component {
            std::path::Component::Prefix(prefix) => match prefix.kind() {
                std::path::Prefix::VerbatimDisk(disk) | std::path::Prefix::Disk(disk) => {
                    path.push(disk.to_ascii_lowercase() as char)
                }
                _ => return None,
            },
            std::path::Component::Normal(c) => {
                path.push_str(c.to_str()?);
            }
            std::path::Component::RootDir => {}
            std::path::Component::CurDir => path.push('.'),
            std::path::Component::ParentDir => path.push_str(".."),
        }
    }
    Some(path.into_boxed_str())
}
