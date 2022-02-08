use std::{
    io,
    time::{Duration, Instant},
};

use proc_maps::{MapRange, Pid};
use read_process_memory::{CopyAddress, ProcessHandle};
use snafu::{OptionExt, ResultExt, Snafu};
use sysinfo::{self, PidExt, ProcessExt};

use crate::{runtime::ProcessList, signature::Signature};

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
    #[cfg(windows)]
    windows_handle: WindowsHandle,
}

#[cfg(windows)]
struct WindowsHandle(winapi::um::winnt::HANDLE);

#[cfg(windows)]
impl Drop for WindowsHandle {
    fn drop(&mut self) {
        unsafe {
            winapi::um::handleapi::CloseHandle(self.0);
        }
    }
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

        #[cfg(windows)]
        let windows_handle = unsafe {
            winapi::um::processthreadsapi::OpenProcess(
                winapi::um::winnt::PROCESS_VM_READ | winapi::um::winnt::PROCESS_QUERY_INFORMATION,
                false as _,
                pid,
            )
        };
        #[cfg(windows)]
        if windows_handle.is_null() {
            return Err(OpenError::InvalidHandle {
                source: io::Error::last_os_error(),
            });
        }

        Ok(Process {
            handle,
            pid,
            modules: Vec::new(),
            last_check: Instant::now() - Duration::from_secs(1),
            #[cfg(windows)]
            windows_handle: WindowsHandle(windows_handle),
        })
    }

    pub fn is_open(&self, process_list: &mut ProcessList) -> bool {
        // FIXME: We can actually ask the list to only refresh the individual process.
        process_list.refresh();
        process_list.is_open(sysinfo::Pid::from_u32(self.pid as u32))
    }

    pub fn module_address(&mut self, module: &str) -> Result<Address, ModuleError> {
        self.refresh_modules().context(ListModules)?;
        self.modules
            .iter()
            .find(|m| m.filename().map_or(false, |f| f.ends_with(module)))
            .context(ModuleDoesntExist)
            .map(|m| m.start() as u64)
    }

    fn refresh_modules(&mut self) -> Result<(), io::Error> {
        let now = Instant::now();
        if now - self.last_check >= Duration::from_secs(1) {
            self.modules = match proc_maps::get_process_maps(self.pid) {
                Ok(m) => m,
                Err(source) => {
                    self.modules.clear();
                    return Err(source);
                }
            };
            self.last_check = now;
        }
        Ok(())
    }

    pub fn read_mem(&self, address: Address, buf: &mut [u8]) -> io::Result<()> {
        self.handle.copy_address(address as usize, buf)
    }

    pub fn scan_signature(&mut self, signature: &Signature) -> io::Result<Option<Address>> {
        let (regions, handle) = self.iter_signature_regions()?;
        let mut vec = Vec::new();
        for [addr, len] in regions {
            eprintln!("{addr:016x?}, {len}");
            if len > vec.len() {
                vec.resize(len, 0);
            }
            let buf = &mut vec[..len];
            if handle.copy_address(addr, buf).is_ok() {
                if let Some(offset) = signature.scan(buf) {
                    return Ok(Some(addr as Address + offset as Address));
                }
            }
        }
        Ok(None)
    }

    #[cfg(not(windows))]
    fn iter_signature_regions(
        &mut self,
    ) -> io::Result<(impl Iterator<Item = [usize; 2]> + '_, &mut ProcessHandle)> {
        self.refresh_modules()?;
        Ok((
            self.modules.iter().map(|m| [m.start(), m.size()]),
            &mut self.handle,
        ))
    }

    #[cfg(windows)]
    fn iter_signature_regions(
        &mut self,
    ) -> io::Result<(impl Iterator<Item = [usize; 2]> + '_, &mut ProcessHandle)> {
        use core::mem;

        use winapi::um::{
            memoryapi::VirtualQueryEx,
            winnt::{MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS},
        };

        // hardcoded values because GetSystemInfo / GetNativeSystemInfo can't
        // return info for remote process
        let is_64bit;
        #[cfg(target_pointer_width = "64")]
        {
            use winapi::{shared::minwindef::BOOL, um::wow64apiset::IsWow64Process};

            let mut pbool: BOOL = 0;
            unsafe { IsWow64Process(self.windows_handle.0, &mut pbool) };
            is_64bit = pbool == 0;
        }
        #[cfg(not(target_pointer_width = "64"))]
        {
            // TODO: Actually idk if 32-bit apps can read from 64-bit
            // apps. If they can, then this is wrong.
            is_64bit = false;
        }

        let min = 0x10000u64;
        let max = if is_64bit {
            0x00007FFFFFFEFFFFu64
        } else {
            0x7FFEFFFFu64
        };

        let mut addr = min;

        const MBI_SIZE: usize = mem::size_of::<MEMORY_BASIC_INFORMATION>();
        let handle = &mut self.windows_handle;
        let iter = core::iter::from_fn(move || {
            while addr < max {
                unsafe {
                    let mut mbi = mem::MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();
                    if VirtualQueryEx(handle.0, addr as _, mbi.as_mut_ptr(), MBI_SIZE) == 0 {
                        break;
                    }
                    let mbi = mbi.assume_init_ref();
                    addr += mbi.RegionSize as u64;

                    // We don't care about reserved / free pages
                    if mbi.State != MEM_COMMIT {
                        continue;
                    }

                    // We can't read from guarded pages
                    if (mbi.Protect & PAGE_GUARD) != 0 {
                        continue;
                    }

                    // We can't read from no access pages
                    if (mbi.Protect & PAGE_NOACCESS) != 0 {
                        continue;
                    }

                    return Some([mbi.BaseAddress as usize, mbi.RegionSize as usize]);
                }
            }
            None
        });

        Ok((iter, &mut self.handle))
    }
}
