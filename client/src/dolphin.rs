use log::{debug, trace};
use proc_maps::get_process_maps;
use sysinfo::{PidExt, ProcessExt, System, SystemExt};

pub fn hook_it() {
    let mut sys = System::new();
    sys.refresh_processes();

    let dol = sys.processes_by_name("dolphin").next().unwrap();
    let pid = dol.pid().as_u32();
    trace!("Dolphin found with pid {pid}");

    let maps = get_process_maps(pid as proc_maps::Pid).unwrap();
    let map = maps.iter().find(|m| {
        m.size() == 0x2000000
            && m.filename()
                .and_then(|f| Some(f.to_string_lossy().contains("dolphin-emu")))
                .unwrap_or(false)
    });
    debug!("Found emulated memory region {map:#?}");
}
