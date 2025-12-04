use log::info;

// https://pcisig.com/sites/default/files/files/PCI_Code-ID_r_1_11__v24_Jan_2019.pdf

#[repr(C)]
struct PciConfig {
    vendor_id: u16,
    device_id: u16,
    command: u16,
    status: u16,
    revision_id: u8,
    prog_if: u8,
    subclass: u8,
    class_code: u8,
    // other fields omitted...
}

pub fn get_pci_base_address(fdt: &fdt::Fdt) -> Result<usize, &'static str> {
    let Some(pci) = fdt.find_compatible(&["pci-host-ecam-generic"]) else {
        info!("No pci-host-ecam-generic controller found");
        return Err("No pci-host-ecam-generic controller found");
    };

    info!("Found PCIe ECAM root: {}", pci.name);

    let reg = pci.reg().unwrap().next().unwrap();
    Ok(reg.starting_address as usize)
}

fn read_pci_class_id(base_addr: usize) -> (u8, u8, u16) {
    // Safety: assumes base_addr points to a valid PCIe config header
    let cfg: &PciConfig = unsafe { &*(base_addr as *const PciConfig) };
    (cfg.class_code, cfg.subclass, cfg.device_id)
}

pub fn scan_pci_devices(base_addr: usize) {
    for bus in 0..=255 {
        for device in 0..32 {
            for function in 0..8 {
                let cfg_addr = base_addr
                    + ((bus as usize) << 20)
                    + ((device as usize) << 15)
                    + ((function as usize) << 12);
                let vendor_id = unsafe { core::ptr::read_volatile(cfg_addr as *const u16) };
                if vendor_id == 0xFFFF {
                    continue;
                }

                let (class_code, subclass, device_id) = read_pci_class_id(cfg_addr);
                info!(
                    "Found PCI Device - Bus: {}, Device: {}, Function: {}, Class Code: {:#X}, Subclass: {:#X}, Device ID: {:#X}",
                    bus,
                    device,
                    function,
                    class_code,
                    subclass,
                    device_id
                );
                if class_code == 0x01 && subclass == 0x08 {
                    // NVMe device found
                    log::info!(
                        "Found NVMe Device - Bus: {}, Device: {}, Function: {}, Device ID: {:#X}",
                        bus,
                        device,
                        function,
                        device_id
                    );
                }
            }
        }
    }
}
