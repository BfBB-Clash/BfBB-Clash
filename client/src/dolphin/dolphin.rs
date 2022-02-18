use clash::{room::Room, spatula::Spatula};
use log::{debug, error, trace};
use process_memory::{Memory, ProcessHandle, TryIntoProcessHandle};
use sysinfo::{PidExt, ProcessExt, System, SystemExt};
use thiserror::Error;

use crate::game::GameInterface;

use super::DataMember;

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

    pub fn unhook(&mut self) {
        self.base_address = None;
        self.handle = None;
    }
}

const LOADING_ADDRESS: usize = 0x803CB7B0;
const WHEREAMI_ADDRESS: usize = 0x803CB8A8;
const SCENE_PTR_ADDRESS: usize = 0x803C2518;
const SPATULA_COUNT_ADDRESS: usize = 0x803C205C;
const SWORLD_BASE: usize = 0x802F63C8;
const LAB_DOOR_ADDRESS: usize = 0x804F6CB8;

// TODO: Don't panic when dolphin isn't hooked
// TODO: Cache DataMembers; they contain a Vec so it isn't the best idea to be making new ones
//       every time we interact with the game.
impl GameInterface for Dolphin {
    fn is_loading(&self) -> bool {
        let ptr = DataMember::<u32>::new_offset(
            self.handle.unwrap(),
            self.base_address.unwrap(),
            vec![LOADING_ADDRESS],
        );
        ptr.read().unwrap().swap_bytes() != 0
    }

    fn start_new_game(&self) {
        let ptr = DataMember::<u32>::new_offset(
            self.handle.unwrap(),
            self.base_address.unwrap(),
            vec![WHEREAMI_ADDRESS],
        );
        ptr.write(&12u32.to_be()).unwrap();
    }

    fn get_current_level(&self) -> Room {
        let base = self.base_address.unwrap();
        let ptr = DataMember::<[u8; 4]>::new_offset(
            self.handle.unwrap(),
            base,
            vec![SCENE_PTR_ADDRESS, 0],
        );

        ptr.read().unwrap().try_into().unwrap()
    }

    fn get_spatula_count(&self) -> u32 {
        let ptr = DataMember::<u32>::new_offset(
            self.handle.unwrap(),
            self.base_address.unwrap(),
            vec![SPATULA_COUNT_ADDRESS],
        );

        ptr.read().unwrap().swap_bytes()
    }

    fn set_spatula_count(&self, value: u32) {
        let ptr = DataMember::<u32>::new_offset(
            self.handle.unwrap(),
            self.base_address.unwrap(),
            vec![SPATULA_COUNT_ADDRESS],
        );
        ptr.write(&value.to_be()).unwrap();
    }

    fn mark_task_complete(&self, spatula: Spatula) {
        let (world_idx, idx) = spatula.into();

        let handle = self.handle.unwrap();

        // TODO: reduce magic numbers
        let mut base = SWORLD_BASE;
        base += world_idx as usize * 0x24C;
        base += 0xC;
        base += idx as usize * 0x48;
        base += 0x14;

        let ptr =
            DataMember::<u16>::new_offset(handle, self.base_address.unwrap(), vec![base, 0x14]);
        ptr.write(&2u16.to_be()).unwrap();
    }

    fn is_task_complete(&self, spatula: Spatula) -> bool {
        let (world_idx, idx) = spatula.into();

        let handle = self.handle.unwrap();

        // TODO: reduce magic numbers
        let mut base = SWORLD_BASE;
        base += world_idx as usize * 0x24C;
        base += 0xC;
        base += idx as usize * 0x48;
        base += 0x14;

        let ptr =
            DataMember::<u16>::new_offset(handle, self.base_address.unwrap(), vec![base, 0x14]);
        ptr.read().unwrap().swap_bytes() == 2
    }

    fn is_spatula_being_collected(&self, spatula: Spatula) -> bool {
        let handle = self.handle.unwrap();

        // TODO: reduce magic numbers
        let offset = spatula.get_offset() as usize * 4;

        let ptr = DataMember::<u32>::new_offset(
            handle,
            self.base_address.unwrap(),
            vec![SCENE_PTR_ADDRESS, 0x78, offset, 0x16C],
        );

        ptr.read().unwrap().swap_bytes() & 4 != 0
    }

    fn set_lab_door(&self, value: u32) {
        let ptr = DataMember::<u32>::new_offset(
            self.handle.unwrap(),
            self.base_address.unwrap(),
            vec![LAB_DOOR_ADDRESS],
        );

        // The game uses a greater than check so we need to subtract three instead of two
        let cost = value - 3;
        ptr.write(&cost.to_be()).unwrap();
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
