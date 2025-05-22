#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

use alloc::string::ToString;

use rdrive::get_dev;
use somehal::driver::{block::*, intc::Box, DriverGeneric};

use sdmmc::BLOCK_SIZE;
use sdmmc::err::SdError;
use sdmmc::emmc::clock::{init_global_clk, Clk, ClkError};

pub use sdmmc::{set_impl, Kernel};
pub use sdmmc::emmc::EMmcHost;

const OFFSET: usize = 0x7_A000;

/// Maps `SdError` values from the lower-level driver to generic `DevError`s.
fn deal_emmc_err(err: SdError) -> ErrorBase {
    match err {
        SdError::Timeout | SdError::DataTimeout => ErrorBase::Again,
        SdError::Crc | SdError::EndBit | SdError::Index |
        SdError::DataCrc | SdError::DataEndBit |
        SdError::DataError => ErrorBase::Io,
        SdError::BusPower | SdError::CurrentLimit => ErrorBase::Io,
        SdError::Acmd12Error | SdError::AdmaError => ErrorBase::Io,
        SdError::InvalidResponse | SdError::InvalidResponseType => ErrorBase::InvalidArg { 
            name: "response", 
            val: "invalid".to_string() 
        },
        SdError::NoCard => ErrorBase::Busy,
        SdError::UnsupportedCard => ErrorBase::InvalidArg { 
            name: "card", 
            val: "unsupported".to_string() 
        },
        SdError::IoError | SdError::TransferError => ErrorBase::Io,
        SdError::CommandError => ErrorBase::Io,
        SdError::TuningFailed | SdError::VoltageSwitchFailed => ErrorBase::InvalidArg { 
            name: "configuration", 
            val: "failed".to_string() 
        },
        SdError::BadMessage | SdError::InvalidArgument => ErrorBase::InvalidArg { 
            name: "parameter", 
            val: "invalid".to_string() 
        },
        SdError::BufferOverflow | SdError::MemoryError => ErrorBase::NoMem,
        SdError::BusWidth => ErrorBase::InvalidArg { 
            name: "bus_width", 
            val: "invalid".to_string() 
        },
        SdError::CardError(_, _) => ErrorBase::InvalidArg { 
            name: "card_error", 
            val: "card operation failed".to_string() 
        },
    }
}

/// Driver for the RK3568 eMMC controller.
pub struct EmmcDriver(pub EMmcHost);

impl EmmcDriver {
    /// Creates a new `EmmcDriver` instance.
    pub fn new(base_addr: usize) -> Self {
        EmmcDriver(EMmcHost::new(base_addr))
    }
}

impl DriverGeneric for EmmcDriver {
    fn open(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }
}

impl Interface for EmmcDriver {
    /// Reads a single block from the eMMC device into the provided buffer.
    fn read_block(&mut self, block_id: u64, buf: &mut [u8]) -> Result<(), ErrorBase> {
        let block_id = block_id + OFFSET as u64;
        if buf.len() < BLOCK_SIZE {
            return Err(ErrorBase::InvalidArg { 
                name: "buffer", 
                val: "size too small".to_string() 
            });
        }

        let (prefix, _, suffix) = unsafe { buf.align_to_mut::<u32>() };
        if !prefix.is_empty() || !suffix.is_empty() {
            return Err(ErrorBase::InvalidArg { 
                name: "buffer", 
                val: "not aligned to u32".to_string() 
            });
        }

        self.0
            .read_blocks(block_id as u32, 1, buf)
            .map_err(deal_emmc_err)
    }

    /// Writes a single block to the eMMC device from the given buffer.
    fn write_block(&mut self, block_id: u64, buf: &[u8]) -> Result<(), ErrorBase> {
        let block_id = block_id + OFFSET as u64;
        if buf.len() < BLOCK_SIZE {
            return Err(ErrorBase::InvalidArg { 
                name: "buffer", 
                val: "size too small".to_string() 
            });
        }

        let (prefix, _, suffix) = unsafe { buf.align_to::<u32>() };
        if !prefix.is_empty() || !suffix.is_empty() {
            return Err(ErrorBase::InvalidArg { 
                name: "buffer", 
                val: "not aligned to u32".to_string() 
            });
        }

        self.0
            .write_blocks(block_id as u32, 1, buf)
            .map_err(deal_emmc_err)
    }

    /// Flushes any cached writes (no-op for now).
    fn flush(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }

    /// Returns the total number of blocks available on the device.
    #[inline]
    fn num_blocks(&self) -> u64 {
        self.0.get_block_num()
    }

    /// Returns the block size in bytes.
    #[inline]
    fn block_size(&self) -> usize {
        self.0.get_block_size()
    }
}

pub struct EmmcClk {
    pub core_clk_index: usize,
}

impl EmmcClk {
    pub fn new(core_clk_index: usize) -> Self {
        EmmcClk { core_clk_index }
    }
}

impl Clk for EmmcClk {
    fn emmc_get_clk(&self) -> Result<u64, ClkError> {
        let clk = get_dev!(Clk).unwrap();
        let clk_dev = clk.spin_try_borrow_by(0.into()).unwrap();
        let rate = clk_dev.get_rate(self.core_clk_index.into()).unwrap();
        Ok(rate)
    }

    fn emmc_set_clk(&self, rate: u64) -> Result<u64, ClkError> {
        let clk = get_dev!(Clk).unwrap();
        let mut clk_dev = clk.spin_try_borrow_by(0.into()).unwrap();
        let _rate = clk_dev.set_rate(self.core_clk_index.into(), rate).unwrap();
        Ok(0)
    }
}

pub fn init_clk(core_clk_index: usize) -> Result<(), ClkError> {
    let emmc_clk = EmmcClk::new(core_clk_index);
    let static_clk: &'static dyn Clk = Box::leak(Box::new(emmc_clk));
    init_global_clk(static_clk);
    Ok(())
}