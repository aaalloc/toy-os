/// NVMe Spec 4.2
/// Submission queue entry
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, packed)]
pub struct NVMeCommand {
    /// Opcode
    pub opcode: u8,
    /// Flags; FUSE (2 bits) | Reserved (4 bits) | PSDT (2 bits)
    pub flags: u8,
    /// Command ID
    pub c_id: u16,
    /// Namespace ID
    pub ns_id: u32,
    /// Reserved
    pub _rsvd: u64,
    /// Metadata pointer
    pub md_ptr: u64,
    /// Data pointer
    pub d_ptr: [u64; 2],
    /// Command dword 10
    pub cdw10: u32,
    /// Command dword 11
    pub cdw11: u32,
    /// Command dword 12
    pub cdw12: u32,
    /// Command dword 13
    pub cdw13: u32,
    /// Command dword 14
    pub cdw14: u32,
    /// Command dword 15
    pub cdw15: u32,
}

impl NVMeCommand {
    #[inline]
    fn new(opcode: u8, c_id: u16) -> Self {
        Self {
            opcode,
            c_id,
            ..Default::default()
        }
    }

    #[inline]
    fn with_prp(mut self, ptr0: u64, ptr1: u64) -> Self {
        self.d_ptr = [ptr0, ptr1];
        self
    }

    #[inline]
    fn with_cdw10(mut self, v: u32) -> Self {
        self.cdw10 = v;
        self
    }

    #[inline]
    fn with_cdw11(mut self, v: u32) -> Self {
        self.cdw11 = v;
        self
    }

    #[inline]
    fn with_ns_id(mut self, ns_id: u32) -> Self {
        self.ns_id = ns_id;
        self
    }

    pub fn create_io_completion_queue(c_id: u16, qid: u16, ptr: usize, size: u16) -> Self {
        Self::new(0x05, c_id)
            .with_prp(ptr as u64, 0)
            .with_cdw10(((size as u32) << 16) | (qid as u32))
            .with_cdw11(1)
    }

    pub fn create_io_submission_queue(
        c_id: u16,
        q_id: u16,
        ptr: usize,
        size: u16,
        cq_id: u16,
    ) -> Self {
        // TODO: QPRIO and NVMESETID
        Self::new(1, c_id)
            .with_prp(ptr as u64, 0)
            .with_cdw10(((size as u32) << 16) | (q_id as u32))
            .with_cdw11(((cq_id as u32) << 16) | 1) // Physically Contiguous
    }

    pub fn delete_io_submission_queue(c_id: u16, q_id: u16) -> Self {
        Self::new(0, c_id).with_cdw10(q_id as u32)
    }

    pub fn delete_io_completion_queue(c_id: u16, q_id: u16) -> Self {
        Self::new(0x04, c_id).with_cdw10(q_id as u32)
    }

    pub fn identify_namespace(c_id: u16, ptr: usize, ns_id: u32) -> Self {
        Self::new(6, c_id).with_ns_id(ns_id).with_prp(ptr as u64, 0)
    }

    pub fn identify_controller(c_id: u16, ptr: usize) -> Self {
        Self::new(6, c_id).with_prp(ptr as u64, 0).with_cdw10(1)
    }

    pub fn identify_namespace_list(c_id: u16, ptr: usize, base: u32) -> Self {
        Self::new(6, c_id)
            .with_prp(ptr as u64, 0)
            .with_cdw10(2)
            .with_ns_id(base)
    }

    pub fn get_features(c_id: u16, ptr: usize, fid: u8) -> Self {
        // TODO: SEL
        Self::new(0x0A, c_id)
            .with_prp(ptr as u64, 0)
            .with_cdw10(u32::from(fid))
    }

    pub fn io_read(c_id: u16, ns_id: u32, lba: u64, blocks_1: u16, ptr0: u64, ptr1: u64) -> Self {
        Self {
            opcode: 2,
            flags: 0,
            c_id,
            ns_id,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [ptr0, ptr1],
            cdw10: lba as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: blocks_1 as u32,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn io_write(c_id: u16, ns_id: u32, lba: u64, blocks_1: u16, ptr0: u64, ptr1: u64) -> Self {
        Self {
            opcode: 1,
            flags: 0,
            c_id,
            ns_id,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [ptr0, ptr1],
            cdw10: lba as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: blocks_1 as u32,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub(crate) fn format_nvm(c_id: u16, ns_id: u32) -> Self {
        Self {
            opcode: 0x80,
            flags: 0,
            c_id,
            ns_id,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [0, 0],
            cdw10: 1 << 9,
            // TODO: dealloc and prinfo bits
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub(crate) fn async_event_req(c_id: u16) -> Self {
        Self {
            opcode: 0xC,
            flags: 0,
            c_id,
            ns_id: 0,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [0, 0],
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub(crate) fn get_log_page(
        c_id: u16,
        numd: u32,
        ptr0: u64,
        ptr1: u64,
        lid: u8,
        lpid: u16,
    ) -> Self {
        Self {
            c_id,
            d_ptr: [ptr0, ptr1],
            cdw10: (numd << 16) | lid as u32,
            cdw11: ((lpid as u32) << 16) | numd >> 16,
            ..Self::default()
        }
    }

    // not supported by samsung
    pub fn write_zeroes(c_id: u16, ns_id: u32, slba: u64, nlb: u16, deac: bool) -> Self {
        Self {
            opcode: 8,
            flags: 0,
            c_id,
            ns_id,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [0, 0],
            cdw10: slba as u32,
            // TODO: dealloc and prinfo bits
            cdw11: (slba >> 32) as u32,
            cdw12: ((deac as u32) << 25) | nlb as u32,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }
}
