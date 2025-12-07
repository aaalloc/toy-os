use core::fmt::{Display, Formatter};
use core::mem::offset_of;

use alloc::fmt;
use log::info;

// https://pcisig.com/sites/default/files/files/PCI_Code-ID_r_1_11__v24_Jan_2019.pdf

use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::registers::{ReadOnly, ReadWrite};
use tock_registers::register_structs;

use tock_registers::register_bitfields;

use crate::memory::KERNEL_SPACE;

register_bitfields! {
    u16, 
    PciCommand [
        IO_SPACE OFFSET(0) NUMBITS(1) [],
        MEMORY_SPACE OFFSET(1) NUMBITS(1) [],
        BUS_MASTER OFFSET(2) NUMBITS(1) [],
        SPECIAL_CYCLES OFFSET(3) NUMBITS(1) [],
        MEM_WRITE_INVALIDATE OFFSET(4) NUMBITS(1) [],
        VGA_PALETTE_SNOOP OFFSET(5) NUMBITS(1) [],
        PARITY_ERROR_RESPONSE OFFSET(6) NUMBITS(1) [],
        SERR OFFSET(8) NUMBITS(1) [],
        FAST_BACK_TO_BACK_ENABLE OFFSET(9) NUMBITS(1) [],
        IRQ OFFSET(10) NUMBITS(1) []
    ],

    PciStatus [
        INTERRUPT_STATUS OFFSET(3) NUMBITS(1) [],
        CAPABILITIES_LIST OFFSET(4) NUMBITS(1) [],
        MHZ66_CAPABLE OFFSET(5) NUMBITS(1) [],
        FAST_BACK_TO_BACK_CAPABLE OFFSET(7) NUMBITS(1) [],
        DATA_PARITY_ERROR_DETECTED OFFSET(8) NUMBITS(1) [],
        DEVSEL_TIMING OFFSET(9) NUMBITS(2) [],
        SIGNATURE_CORRECT OFFSET(11) NUMBITS(1) [],
        RECEIVED_TARGET_ABORT OFFSET(12) NUMBITS(1) [],
        RECEIVED_MASTER_ABORT OFFSET(13) NUMBITS(1) [],
        SENT_TARGET_ABORT OFFSET(14) NUMBITS(1) [],
        SENT_MASTER_ABORT OFFSET(15) NUMBITS(1) []
    ],
}

register_bitfields![u32,
    Bar [
        // Common BAR type fields
        IO_MEM_SPACE OFFSET(0) NUMBITS(1) [
            MEM = 0,
            IO = 1,
        ],

        MEM_TYPE OFFSET(1) NUMBITS(2) [
            TYPE_32BIT = 0,
            TYPE_64BIT = 2,
        ],

        MEM_PREFETCHABLE OFFSET(3) NUMBITS(1) [],
        MEM_BASE_ADDR   OFFSET(4) NUMBITS(27) [],


        IO_BASE_ADDR OFFSET(2) NUMBITS(29) []
    ]
];


register_structs! { 
    pub PciConfig {
        (0x00 => pub vendor_id: ReadOnly<u16>),
        (0x02 => pub device_id: ReadOnly<u16>),
        (0x04 => pub command: ReadWrite<u16, PciCommand::Register>),
        (0x06 => pub status: ReadOnly<u16, PciStatus::Register>),
        (0x08 => pub revision_id: ReadOnly<u8>),
        (0x09 => pub prog_if: ReadOnly<u8>),
        (0x0A => pub subclass: ReadOnly<u8>),
        (0x0B => pub class_code: ReadOnly<u8>),
        (0x0C => pub cache_line_size: ReadWrite<u8>),
        (0x0D => pub latency_timer: ReadWrite<u8>),
        (0x0E => pub header_type: ReadOnly<u8>),
        (0x0F => pub bist: ReadWrite<u8>),
        (0x10 => @END),
    },
    pub PciHeaderType0 {
        (0x00 => pub config: PciConfig),
        (0x10 => pub bar: [ReadWrite<u32, Bar::Register>; 6]),
        (0x28 => pub cis_pointer: ReadWrite<u32>),
        (0x2C => pub sub_vendor_id: ReadOnly<u16>),
        (0x2E => pub sub_device_id: ReadOnly<u16>),
        (0x30 => pub rom_bar: ReadWrite<u32>),
        (0x34 => pub capabilities_pointer: ReadWrite<u8>),
        (0x35 => _reserved1: [u8; 3]),
        (0x38 => pub interrupt_line: ReadWrite<u8>),
        (0x39 => pub interrupt_pin: ReadOnly<u8>),
        (0x3A => pub min_grant: ReadOnly<u8>),
        (0x3B => pub max_latency: ReadOnly<u8>),
        (0x3C => @END),
    },

    // https://wiki.osdev.org/NVMe
    pub NvmeDevice {
        (0x00 => pub cap: ReadOnly<u64>),        // Controller Capabilities
        (0x08 => pub vs: ReadOnly<u32>),         // Version
        (0x0C => pub intms: ReadWrite<u32>),      // Interrupt Mask Set
        (0x10 => pub intmc: ReadWrite<u32>),      // Interrupt Mask Clear
        (0x14 => pub cc: ReadWrite<u32>),         // Controller Configuration
        (0x18 => _rsvd1: [u8; 4]),
        (0x1C => pub csts: ReadOnly<u32>),       // Controller Status
        (0x20 => pub nssr: ReadWrite<u32>),       // NVM Subsystem Reset (optional)
        (0x24 => pub aqa: ReadWrite<u32>),        // Admin Queue Attributes
        (0x28 => pub asq: ReadWrite<u64>),        // Admin Submission Queue Base Address
        (0x30 => pub acq: ReadWrite<u64>),        // Admin Completion Queue Base Address
        // NOTE: not sure
        (0x38 => pub cmbloc: ReadWrite<u32>),     // Controller Memory Buffer Location (optional)
        (0x3C => pub cmbsz: ReadWrite<u32>),      // Controller Memory Buffer Size (optional)
        (0x40 => pub bpinfo: ReadWrite<u32>),     // Boot Partition Information
        (0x44 => pub bprsel: ReadWrite<u32>),     // Boot Partition Read Select
        (0x48 => pub bpmbl: ReadWrite<u64>),      // Boot Partition Memory Buffer Location
        (0x50 => @END),
    }

}

pub enum PciDeviceType {
    Nvme(NvmeDevice),
    Other,
}

impl Display for PciConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
    vendor_id: {:04x},
    device_id: {:04x},
    command: {:04x},
    status: {:04x},
    revision_id: {:02x},
    prog_if: {:02x},
    subclass: {:02x},
    class_code: {:02x},
    cache_line_size: {:02x},
    latency_timer: {:02x},
    header_type: {:02x},
    bist: {:02x},
    ",
            self.vendor_id.get(),
            self.device_id.get(),
            self.command.get(),
            self.status.get(),
            self.revision_id.get(),
            self.prog_if.get(),
            self.subclass.get(),
            self.class_code.get(),
            self.cache_line_size.get(),
            self.latency_timer.get(),
            self.header_type.get(),
            self.bist.get(),
        )
    }
}

// Exemple pour PciHeaderType0
impl Display for PciHeaderType0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
    config: {},
    bar: [{:08x}, {:08x}, {:08x}, {:08x}, {:08x}, {:08x}],
    cis_pointer: {:08x},
    sub_vendor_id: {:04x},
    sub_device_id: {:04x},
    rom_bar: {:08x},
    capabilities_pointer: {:02x},
    interrupt_line: {:02x},
    interrupt_pin: {:02x},
    min_grant: {:02x},
    max_latency: {:02x},
    ",
            self.config,
            self.bar[0].get(),
            self.bar[1].get(),
            self.bar[2].get(),
            self.bar[3].get(),
            self.bar[4].get(),
            self.bar[5].get(),
            self.cis_pointer.get(),
            self.sub_vendor_id.get(),
            self.sub_device_id.get(),
            self.rom_bar.get(),
            self.capabilities_pointer.get(),
            self.interrupt_line.get(),
            self.interrupt_pin.get(),
            self.min_grant.get(),
            self.max_latency.get(),
        )
    }
}


impl PciDeviceType {
    pub fn new(cfg: &PciHeaderType0) -> Option<Self> {
        match (cfg.config.class_code.get(), cfg.config.subclass.get(), cfg.config.prog_if.get()) {
            (0x01, 0x08, 0x02) => {
                let nvme_base = ( cfg as *const PciHeaderType0 as usize + 0x40 ) as *mut NvmeDevice;
                let nvme = unsafe { core::ptr::read_volatile(nvme_base) };
                Some(PciDeviceType::Nvme(nvme))
            }
            _ => None,
        }
    }
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

                let cfg: PciHeaderType0 = unsafe {
                    core::ptr::read_volatile(cfg_addr as *const PciHeaderType0)
                };
                
                // info!("{}", cfg);
                
                match PciDeviceType::new(&cfg) {
                    Some(PciDeviceType::Nvme(nvme)) => {
                        info!("Found NVMe PCI Device at {:02x}:{:02x}.{:x}", bus, device, function);
                        info!("Device Config: {}", cfg);

                        // Enable interrupts, bus-mastering DMA, and memory space access in the PCI configuration space for the function.
                        cfg.config.command.write(PciCommand::IRQ::SET);
                        cfg.config.command.write(PciCommand::BUS_MASTER::SET);
                        cfg.config.command.write(PciCommand::MEMORY_SPACE::SET);
                        cfg.config.command.write(PciCommand::IO_SPACE::SET);


                        // 32 or 64bit memory space bar
                        let bar0_type: Bar::MEM_TYPE::Value = cfg.bar[0].read_as_enum(Bar::MEM_TYPE).unwrap();
                        assert_eq!(bar0_type, Bar::MEM_TYPE::Value::TYPE_64BIT);

                        let something: Option<Bar::IO_MEM_SPACE::Value> = cfg.bar[0].read_as_enum(Bar::IO_MEM_SPACE);
                        match something {
                            Some(Bar::IO_MEM_SPACE::Value::IO) => info!("bar0 is io"),
                            Some(Bar::IO_MEM_SPACE::Value::MEM) => info!("bar0 is mem"),
                            None => panic!("not io or mem"),
                        }

                        // let bar0_prev = cfg.bar[0].get();
                        // info!("  BAR0 Original Value: {:08x}", bar0_prev);
                        // cfg.bar[0].set(0xFFFF_FFFF);    
                        // let bar0_size = !(cfg.bar[0].get() & 0xFFFF_FFF0) + 1;
                        // info!("  BAR0 Size: {:08x}, {}", bar0_size, bar0_size as usize); 
                        // cfg.bar[0].set(bar0_prev);

                        // now map the BAR0 to some MMIO region
                        // let bar0_base_addr = 0x4000_0000;
                        // KERNEL_SPACE.exclusive_access().map_mmio(
                        //     bar0_base_addr,
                        //     bar0_base_addr + bar0_size as usize
                        // );

                        // cfg.bar[0].set((mmio_base & 0xFFFF_FFF0) as u32);
                        // cfg.bar[1].set((mmio_base >> 32) as u32);
                        // nvme_base_addr = (uint64_t)(((uint64_t)bar1 << 32) | (bar0 & 0xFFFFFFF0));
                        let nvme_base_addr = ((cfg.bar[1].get() as u64) << 32) | ((cfg.bar[0].get() & 0xFFFF_FFF0) as u64);
                        info!("  NVMe Base Address: {:016x}", nvme_base_addr);
                        info!("Device Config: {}", cfg);

                        // let nvme_ptr = nvme_base_addr as *mut NvmeDevice;
                        // let nvme_dev = unsafe { core::ptr::read_volatile(nvme_ptr) };
                        // info!("  NVMe Device Capabilities: {:016x}", nvme_dev.cap.get());
                        // info!("  NVMe Device Version: {:08x}", nvme_dev.vs.get());
                    }
                    Some(_) => {}
                    None => {}
                }
            }
        }
    }
}
