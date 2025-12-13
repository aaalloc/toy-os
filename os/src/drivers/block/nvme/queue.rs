use crate::drivers::block::nvme::{
    cmd::NVMeCommand,
    dma::{DMAError, Dma},
};
pub const QUEUE_LENGTH: usize = 256;

pub struct NVMeSubmissionQueue {
    commands: Dma<[NVMeCommand; QUEUE_LENGTH]>,
    pub head: usize,
    pub tail: usize,
    len: usize,
    pub doorbell: usize,
}

impl NVMeSubmissionQueue {
    pub fn new(doorbell: usize) -> Result<Self, DMAError> {
        Ok(Self {
            commands: Dma::new()?,
            head: 0,
            tail: 0,
            len: QUEUE_LENGTH,
            doorbell,
        })
    }

    pub const fn size(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        self.head == (self.tail + 1) % self.len
    }

    pub fn submit_checked(&mut self, entry: NVMeCommand) -> Option<usize> {
        if self.is_full() {
            None
        } else {
            Some(self.submit(entry))
        }
    }

    #[inline(always)]
    pub fn submit(&mut self, entry: NVMeCommand) -> usize {
        // println!("SUBMISSION ENTRY: {:?}", entry);
        self.commands[self.tail] = entry;

        self.tail = (self.tail + 1) % self.len;
        self.tail
    }

    pub fn get_addr(&self) -> usize {
        self.commands.paddr().0 as usize
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C, packed)]
pub struct NVMeCompletion {
    pub command_specific: u32,
    pub _rsvd: u32,
    pub submission_queue_head: u16,
    pub submission_queue_id: u16,
    pub command_id: u16,
    pub status: u16,
}

pub struct NVMeCompletionQueue {
    commands: Dma<[NVMeCompletion; QUEUE_LENGTH]>,
    head: usize,
    phase: bool,
    len: usize,
    pub doorbell: usize,
}

impl NVMeCompletionQueue {
    pub fn new(doorbell: usize) -> Result<Self, DMAError> {
        Ok(Self {
            commands: Dma::new()?,
            head: 0,
            phase: true,
            len: QUEUE_LENGTH,
            doorbell,
        })
    }

    pub const fn size(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn complete(&mut self) -> Option<(usize, NVMeCompletion, usize)> {
        let entry = &self.commands[self.head];

        if ((entry.status & 1) == 1) == self.phase {
            let prev = self.head;
            self.head = (self.head + 1) % self.len;
            if self.head == 0 {
                self.phase = !self.phase;
            }
            Some((self.head, entry.clone(), prev))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn complete_n(&mut self, commands: usize) -> (usize, NVMeCompletion, usize) {
        let prev = self.head;
        self.head += commands - 1;
        if self.head >= self.len {
            self.phase = !self.phase;
        }
        self.head %= self.len;

        let (head, entry, _) = self.complete_spin();
        (head, entry, prev)
    }

    #[inline(always)]
    pub fn complete_spin(&mut self) -> (usize, NVMeCompletion, usize) {
        loop {
            if let Some(val) = self.complete() {
                return val;
            }
            core::hint::spin_loop();
        }
    }

    pub fn get_addr(&self) -> usize {
        self.commands.paddr().0 as usize
    }
}
