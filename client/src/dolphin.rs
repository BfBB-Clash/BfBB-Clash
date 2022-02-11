use std::num::NonZeroU32;

use log::{debug, error, trace};
use sysinfo::{PidExt, ProcessExt, System, SystemExt};
use thiserror::Error;

const REGION_SIZE: usize = 0x2000000;

// This is a little odd because processes_by_name is case-sensitive
#[cfg(unix)]
const PROCESS_NAME: &str = "dolphin";
#[cfg(windows)]
const PROCESS_NAME: &str = "Dolphin";

#[derive(Debug, Error)]
pub enum Error {
    #[error("The emulated memory region could not be found. Make sure Dolphin is running and the game is open.")]
    RegionNotFound,
}

pub struct Dolphin {
    system: System,
    base_address: Option<usize>,
    pid: Option<NonZeroU32>,
}

impl Default for Dolphin {
    fn default() -> Self {
        Self {
            system: System::new(),
            base_address: None,
            pid: None,
        }
    }
}

impl Dolphin {
    pub fn is_hooked(&self) -> bool {
        self.base_address.is_some()
    }

    pub fn hook(&mut self) -> Result<(), Error> {
        if self.is_hooked() {
            return Ok(());
        }
        self.system.refresh_processes();

        let procs = self.system.processes_by_name(PROCESS_NAME);
        for proc in procs {
            let pid = proc.pid().as_u32();
            trace!("{} found with pid {pid}", proc.name());

            if let Some(addr) = get_emulated_base_address(pid) {
                debug!("Found emulated memory region at {addr:#X}");
                self.base_address = Some(addr);
                self.pid = Some(pid.try_into().expect("Dolphin pid was 0"));
                return Ok(());
            }
        }

        Err(Error::RegionNotFound)
    }
}

#[cfg(unix)]
fn get_emulated_base_address(pid: u32) -> Option<usize> {
    use proc_maps::get_process_maps;
    let maps = match get_process_maps(pid as proc_maps::Pid) {
        Err(e) => {
            error!("Could not get dolphin process maps\n{e:?}");
            return None;
        }
        Ok(maps) => maps,
    };

    let map = maps.iter().find(|m| {
        m.size() == REGION_SIZE
            && m.filename()
                .and_then(|f| Some(f.to_string_lossy().contains("dolphin-emu")))
                .unwrap_or(false)
    });
    map.map(|m| m.start())
}

#[cfg(windows)]
fn get_emulated_base_address(pid: u32) -> Option<usize> {
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::memoryapi::VirtualQueryEx;
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::psapi::{QueryWorkingSetEx, PSAPI_WORKING_SET_EX_INFORMATION};
    use winapi::um::winnt::{
        MEMORY_BASIC_INFORMATION, MEM_MAPPED, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION,
        PROCESS_VM_READ, PROCESS_VM_WRITE,
    };

    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
            0,
            pid,
        );
        if handle.is_null() {
            // TODO use GetLastError for error feedback
            error!("Failed to open process handle for dolphin (pid {pid})");
            return None;
        }

        // Begin memory scan for Dolphin's emulated memory region
        // We are looking for a MEM_MAPPED region of size 0x2000000
        let mut mem_info = MEMORY_BASIC_INFORMATION::default();
        let mut mem = std::ptr::null();
        while VirtualQueryEx(
            handle,
            mem,
            &mut mem_info,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        ) == std::mem::size_of::<MEMORY_BASIC_INFORMATION>()
        {
            if mem_info.RegionSize == REGION_SIZE && mem_info.Type == MEM_MAPPED {
                let mut ws_info = PSAPI_WORKING_SET_EX_INFORMATION {
                    VirtualAddress: mem_info.BaseAddress,
                    ..Default::default()
                };
                if QueryWorkingSetEx(
                    handle,
                    &mut ws_info as *mut _ as *mut std::ffi::c_void,
                    std::mem::size_of::<PSAPI_WORKING_SET_EX_INFORMATION>()
                        .try_into()
                        .unwrap(),
                ) != 0
                    && ws_info.VirtualAttributes.Valid() != 0
                {
                    return Some(mem_info.BaseAddress as usize);
                }
            }

            mem = mem.add(mem_info.RegionSize);
        }

        CloseHandle(handle);
    }

    None
}
