use clash::spatula::Spatula;
use log::{debug, error, trace};
use process_memory::{CopyAddress, ProcessHandle, PutAddress, TryIntoProcessHandle};
use sysinfo::{PidExt, ProcessExt, System, SystemExt};
use thiserror::Error;

use crate::game_interface::GameInterface;

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
    #[error("Dolphin memory could not be accessed.")]
    IO,
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::IO
    }
}

#[derive(Default)]
pub struct Dolphin {
    system: System,
    base_address: Option<usize>,
    handle: Option<ProcessHandle>,
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
        let (pid, addr) = procs
            .into_iter()
            .map(|p| {
                let pid = p.pid().as_u32();
                trace!("{} found with pid {pid}", p.name());
                (pid, get_emulated_base_address(pid))
            })
            .find(|(_, addr)| addr.is_some())
            .unwrap_or((0, None));

        if let Some(addr) = addr {
            debug!("Found emulated memory region at {addr:#X}");
            self.base_address = Some(addr);
            self.handle = Some(pid.try_into_process_handle()?);
            return Ok(());
        }

        Err(Error::RegionNotFound)
    }
}

// TODO: Don't panic when dolphin isn't hooked
impl GameInterface for Dolphin {
    fn start_new_game(&self) {
        let base = self.base_address.unwrap();
        self.handle
            .unwrap()
            .put_address(base + 0x3CB8A8, &12u32.to_be_bytes())
            .unwrap();
    }

    fn set_spatula_count(&self, value: u32) {
        let base = self.base_address.unwrap();
        self.handle
            .unwrap()
            .put_address(base + 0x3C205C, &value.to_be_bytes())
            .unwrap();
    }

    fn mark_task_complete(&self, spatula: Spatula) {
        let (world_idx, idx) = spatula.into();

        let handle = self.handle.unwrap();
        let base = self.base_address.unwrap();
        let world_addr = base + 0x2F63C8 + world_idx as usize * 0x24C;
        let taskarr_addr = world_addr + 0xC;
        let task_addr = taskarr_addr + idx as usize * 0x48;
        let counter_addr = task_addr + 20;
        let mut counter_ptr = [0u8; 4];
        handle.copy_address(counter_addr, &mut counter_ptr).unwrap();
        let counter_ptr =
            u32::from_be_bytes(counter_ptr) as usize - 0x80000000 + self.base_address.unwrap();

        handle
            .put_address(counter_ptr + 20, &2u16.to_be_bytes())
            .unwrap();
    }
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "windows")]
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
