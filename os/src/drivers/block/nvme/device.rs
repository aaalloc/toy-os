use core::error::Error;

use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String};
use log::info;
use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};

use crate::drivers::block::nvme::{
    cmd::NVMeCommand,
    dma::Dma,
    queue::{NVMeCompletion, NVMeCompletionQueue, NVMeSubmissionQueue, QUEUE_LENGTH},
};

// https://files.futurememorystorage.com/proceedings/2013/20130812_PreConfD_Marks.pdf

register_bitfields! [
    // First parameter is the register width. Can be u8, u16, u32, or u64.
    u32,
    VS [
        Major OFFSET(16) NUMBITS(8) [],
        Minor OFFSET(8) NUMBITS(8) [],
        Tertiary OFFSET(0) NUMBITS(8) []
    ],
    CC [
        EN      OFFSET(0)  NUMBITS(1), // Enable
        CSS     OFFSET(4)  NUMBITS(3), // Command Set Selected
        MPS     OFFSET(7)  NUMBITS(4), // Memory Page Size
        AMS     OFFSET(11) NUMBITS(3), // Arbitration Mechanism Selected
        SHN     OFFSET(14) NUMBITS(2), // Shutdown Notification
        IOSQES  OFFSET(16) NUMBITS(4), // I/O Submission Queue Entry Size
        IOCQES  OFFSET(20) NUMBITS(4) // I/O Completion Queue Entry Size
    ],

    CSTS [
        RDY OFFSET(0) NUMBITS(1)
    ]
];

register_bitfields![u64,
    CAP [
        MQES    OFFSET(0)  NUMBITS(16), // Max Queue Entries Supported (0-based)
        CQR     OFFSET(16) NUMBITS(1), // Contiguous Queues Required
        AMS     OFFSET(17) NUMBITS(2), // Arbitration Mechanism Supported
        TO      OFFSET(24) NUMBITS(8), // Timeout
        DSTRD   OFFSET(32) NUMBITS(4), // Doorbell Stride
        NSSRS   OFFSET(36) NUMBITS(1), // NVM Subsystem Reset Supported
        CSS     OFFSET(37) NUMBITS(8), // Command Set Supported
        MPSMIN  OFFSET(48) NUMBITS(4), // Memory Page Size Minimum
        MPSMAX  OFFSET(52) NUMBITS(4), // Memory Page Size Maximum
    ]
];

register_structs! {

    // https://wiki.osdev.org/NVMe
    pub NVMeRegisters {
        (0x00 => pub cap: ReadOnly<u64, CAP::Register>),        // Controller Capabilities
        (0x08 => pub vs: ReadOnly<u32, VS::Register>),         // Version
        (0x0C => pub intms: ReadWrite<u32>),      // Interrupt Mask Set
        (0x10 => pub intmc: ReadWrite<u32>),      // Interrupt Mask Clear
        (0x14 => pub cc: ReadWrite<u32, CC::Register>),         // Controller Configuration
        (0x18 => _rsvd1: [u8; 4]),
        (0x1C => pub csts: ReadOnly<u32, CSTS::Register>),       // Controller Status
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
#[repr(C)]
pub struct NvmeCaps {
    pub mqes: u16,
    pub timeout_ms: u32,
    pub mps_min: u32,
    pub mps_max: u32,
    pub supports_nvm: bool,
}

impl alloc::fmt::Debug for NvmeCaps {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NvmeCaps")
            .field("mqes", &self.mqes)
            .field("timeout_ms", &self.timeout_ms)
            .field("mps_min", &self.mps_min)
            .field("mps_max", &self.mps_max)
            .field("supports_nvm", &self.supports_nvm)
            .finish()
    }
}

impl NVMeRegisters {
    pub fn get_version(&self) -> (u8, u8, u8) {
        let vs = self.vs.get();
        (
            VS::Major.read(vs) as u8,
            VS::Minor.read(vs) as u8,
            VS::Tertiary.read(vs) as u8,
        )
    }

    pub fn read_capabilities(&self) -> NvmeCaps {
        let cap = self.cap.get();

        let mqes = CAP::MQES.read(cap) as u16 + 1;
        let timeout_ms = CAP::TO.read(cap) as u32 * 500;
        let mps_min = 1 << (12 + CAP::MPSMIN.read(cap));
        let mps_max = 1 << (12 + CAP::MPSMAX.read(cap));
        let supports_nvm = (CAP::CSS.read(cap) & 0b00000001) != 0;

        NvmeCaps {
            mqes,
            timeout_ms,
            mps_min,
            mps_max,
            supports_nvm,
        }
    }

    pub fn set_sq_tail(&mut self, qid: u16, val: u32) {
        let doorbell_base = 0x1000;
        let offset = doorbell_base + (qid as usize * 2 * 4);
        unsafe {
            core::ptr::write_volatile(
                (self as *mut NVMeRegisters as usize + offset) as *mut u32,
                val as u32,
            );
        }
    }

    pub fn set_cq_head(&mut self, qid: u16, val: u32) {
        let doorbell_base = 0x1000;
        let offset = doorbell_base + ((qid as usize * 2 + 1) * 4);
        unsafe {
            core::ptr::write_volatile(
                (self as *mut NVMeRegisters as usize + offset) as *mut u32,
                val as u32,
            );
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
#[allow(unused)]
struct IdentifyNamespaceData {
    pub nsze: u64,
    pub ncap: u64,
    nuse: u64,
    nsfeat: u8,
    pub nlbaf: u8,
    pub flbas: u8,
    mc: u8,
    dpc: u8,
    dps: u8,
    nmic: u8,
    rescap: u8,
    fpi: u8,
    dlfeat: u8,
    nawun: u16,
    nawupf: u16,
    nacwu: u16,
    nabsn: u16,
    nabo: u16,
    nabspf: u16,
    noiob: u16,
    nvmcap: u128,
    npwg: u16,
    npwa: u16,
    npdg: u16,
    npda: u16,
    nows: u16,
    _rsvd1: [u8; 18],
    anagrpid: u32,
    _rsvd2: [u8; 3],
    nsattr: u8,
    nvmsetid: u16,
    endgid: u16,
    nguid: [u8; 16],
    eui64: u64,
    pub lba_format_support: [u32; 16],
    _rsvd3: [u8; 192],
    vendor_specific: [u8; 3712],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
#[allow(unused)]
pub struct NVMeNamespace {
    pub id: u32,
    pub blocks: u64,
    pub block_size: u64,
}

impl NVMeNamespace {
    pub fn size_bytes(&self) -> u64 {
        self.blocks * self.block_size
    }
}

pub struct NVMeDevice {
    nvme_dev: &'static mut NVMeRegisters,
    caps: NvmeCaps,
    admin_sq: NVMeSubmissionQueue,
    admin_cq: NVMeCompletionQueue,
    io_sq: NVMeSubmissionQueue,
    io_cq: NVMeCompletionQueue,
    buffer: Dma<[u8; 2 * 1024]>, // 2 MiB buffer
    ns: BTreeMap<u32, NVMeNamespace>,
    q_id: u16,
}

#[allow(unused)]
impl NVMeDevice {
    pub fn new(addr_ptr: usize) -> Result<Self, Box<dyn Error>> {
        let nvme_dev = unsafe { &mut *(addr_ptr as *mut NVMeRegisters) };
        let v = nvme_dev.get_version();
        log::info!("NVMe Controller Version: {}.{}.{}", v.0, v.1, v.2);

        let caps = nvme_dev.read_capabilities();
        let mqes = caps.mqes;
        log::info!("{:?}", caps);
        assert!(
            caps.supports_nvm,
            "NVMe device does not support NVM command set"
        );
        let mut s = NVMeDevice {
            nvme_dev,
            caps,
            admin_sq: NVMeSubmissionQueue::new(0)?,
            admin_cq: NVMeCompletionQueue::new(0)?,
            io_sq: NVMeSubmissionQueue::new(0)?,
            io_cq: NVMeCompletionQueue::new(0)?,
            buffer: Dma::new()?,
            q_id: 1,
            ns: BTreeMap::new(),
        };
        s.init()?;
        for id in s.identify_namespace_list(0) {
            let ns = s.identify_namespace(id);
            log::info!("{:?}, total_size: {}", ns, ns.size_bytes());
            s.ns.insert(id, ns);
        }
        s.identify_controller()?;

        Ok(s)
    }

    pub const fn caps(&self) -> &NvmeCaps {
        &self.caps
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        self.disable();
        self.setup_admin_queues();
        let iosqes = size_of::<NVMeCommand>().trailing_zeros();
        assert!(iosqes == 6); // should be 6 because 2^6 = 64 bytes
        let iocqes = size_of::<NVMeCompletion>().trailing_zeros();
        assert!(iocqes == 4); // should be 4 because 2^4 = 16 bytes
        self.nvme_dev
            .cc
            .modify(CC::IOSQES.val(iosqes) + CC::IOCQES.val(iocqes));

        self.enable();

        let qid = self.q_id;
        let addr = self.io_cq.get_addr();
        info!("Requesting i/o completion queue");
        let comp = self.submit_and_complete_admin(|c_id, _| {
            NVMeCommand::create_io_completion_queue(c_id, qid, addr, (QUEUE_LENGTH - 1) as u16)
        })?;

        let addr = self.io_sq.get_addr();
        info!("Requesting i/o submission queue");
        let comp = self.submit_and_complete_admin(|c_id, _| {
            NVMeCommand::create_io_submission_queue(c_id, qid, addr, (QUEUE_LENGTH - 1) as u16, qid)
        })?;
        self.q_id += 1;
        Ok(())
    }

    fn disable(&mut self) {
        self.nvme_dev.cc.modify(CC::EN::CLEAR);

        while self.nvme_dev.csts.read(CSTS::RDY) != 0 {
            core::hint::spin_loop();
        }
    }

    fn enable(&mut self) {
        self.nvme_dev.cc.modify(CC::EN::SET);

        while self.nvme_dev.csts.read(CSTS::RDY) == 0 {
            core::hint::spin_loop();
        }
    }

    fn setup_admin_queues(&mut self) {
        let aqa = ((self.admin_cq.size() as u32 - 1) << 16) | (self.admin_sq.size() as u32 - 1);
        self.nvme_dev.aqa.set(aqa);
        self.nvme_dev.asq.set(self.admin_sq.get_addr() as u64);
        self.nvme_dev.acq.set(self.admin_cq.get_addr() as u64);
    }

    fn submit_and_complete_admin<F: FnOnce(u16, usize) -> NVMeCommand>(
        &mut self,
        cmd_init: F,
    ) -> Result<NVMeCompletion, Box<dyn Error>> {
        let cid = self.admin_sq.tail as u16;
        let tail = self
            .admin_sq
            .submit(cmd_init(cid as u16, self.buffer.paddr().0 as usize));
        self.nvme_dev.set_sq_tail(0, tail as u32);

        let (head, entry, _) = self.admin_cq.complete_spin();
        self.nvme_dev.set_cq_head(0, head as u32);

        let status = entry.status >> 1;
        if status != 0 {
            info!("Admin command failed with status: {}", status);
            return Err(alloc::format!("Admin command failed with status: {}", status).into());
        }
        Ok(entry)
    }

    pub fn identify_controller(&mut self) -> Result<(), Box<dyn Error>> {
        info!("Trying to identify controller");
        self.submit_and_complete_admin(NVMeCommand::identify_controller)?;

        let data = &self.buffer;

        let mut serial = String::from_utf8(data.as_slice()[4..24].to_vec()).unwrap();
        let mut model = String::from_utf8(data.as_slice()[24..64].to_vec()).unwrap();
        let mut firmware = String::from_utf8(data.as_slice()[64..72].to_vec()).unwrap();

        info!(
            "  -> Model: {}, Serial: {}, Firmware: {}",
            model.trim(),
            serial.trim(),
            firmware.trim()
        );

        Ok(())
    }

    pub fn identify_namespace_list(&mut self, base: u32) -> alloc::vec::Vec<u32> {
        self.submit_and_complete_admin(|c_id, addr| {
            NVMeCommand::identify_namespace_list(c_id, addr, base)
        });

        let data: &[u32] = unsafe {
            core::slice::from_raw_parts(self.buffer.vaddr(0).as_ptr() as *const u32, QUEUE_LENGTH)
        };

        data.iter()
            .copied()
            .take_while(|&id| id != 0)
            .collect::<alloc::vec::Vec<u32>>()
    }

    pub fn identify_namespace(&mut self, id: u32) -> NVMeNamespace {
        self.submit_and_complete_admin(|c_id, addr| {
            NVMeCommand::identify_namespace(c_id, addr, id)
        });

        let namespace_data: IdentifyNamespaceData =
            unsafe { *(self.buffer.vaddr(0).as_ptr() as *const IdentifyNamespaceData) };

        let size = namespace_data.nsze;
        let blocks = namespace_data.ncap;

        // figure out block size
        let flba_idx = (namespace_data.flbas & 0xF) as usize;
        let flba_data = (namespace_data.lba_format_support[flba_idx] >> 16) & 0xFF;
        let block_size = if !(9..32).contains(&flba_data) {
            0
        } else {
            1 << flba_data
        };

        NVMeNamespace {
            id,
            blocks,
            block_size: block_size,
        }
    }

    pub fn read_sync(
        &mut self,
        ns_id: u32,
        lba: u64,
        num_blocks: u16,
    ) -> Result<&[u8], Box<dyn Error>> {
        let ns = self.ns.get(&ns_id).ok_or("Namespace not found")?;

        let cid = self.io_sq.tail as u16;
        let tail = self.io_sq.submit(NVMeCommand::io_read(
            cid,
            ns_id,
            lba,
            num_blocks,
            self.buffer.paddr().0 as u64,
            0,
        ));
        // TODO: self.q_id is wrong, it is 2 but should be 1
        self.nvme_dev.set_sq_tail(1, tail as u32);

        let (head, entry, _) = self.io_cq.complete_spin();
        self.nvme_dev.set_cq_head(1, head as u32);

        let status = entry.status >> 1;
        if status != 0 {
            info!("I/O command failed with status: {}", status);
            return Err(alloc::format!("I/O command failed with status: {}", status).into());
        }

        let size = (num_blocks as u64) * ns.block_size;
        Ok(&self.buffer.as_slice()[0..size as usize])
    }
}
