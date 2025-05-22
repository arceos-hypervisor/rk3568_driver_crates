#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

use somehal::driver::{clk::*, DriverGeneric};

use log::{debug, warn};
use rk3568_clk::RK3568ClkPriv ;
use alloc::string::ToString;
use core::convert::Into;
use core::result::Result::{self, *};
pub struct ClkDriver(RK3568ClkPriv);
pub const EMMC_CLK_ID: usize = 0x7c;

impl ClkDriver {
    pub fn new(cru_address: u64) -> Self {
        ClkDriver (
            unsafe { RK3568ClkPriv ::new(cru_address as *mut _) }
        )
    }
}

impl DriverGeneric for ClkDriver {
    fn open(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }
}

impl Interface for ClkDriver {
    fn perper_enable(&mut self) {
        debug!("perper_enable");    
    }

    fn get_rate(&self, id: ClockId) -> Result<u64, ErrorBase> {
        let rate = match id.into() {
            EMMC_CLK_ID => self.0.emmc_get_clk().unwrap(),
            _ => { 
                warn!("Unsupported clock ID: {:?}", id);
                Err(ErrorBase::InvalidArg { 
                    name: "clock_id", 
                    val: "unsupported".to_string() 
                })?
            }
        };
        Ok(rate as u64)
    }

    fn set_rate(&mut self, id: ClockId, rate: u64) -> Result<(), ErrorBase> {
        match id.into() {
            EMMC_CLK_ID => {
                self.0.emmc_set_clk(rate).map_err(|_| ErrorBase::InvalidArg {
                    name: "emmc_clk",
                    val: "failed to set clock rate".to_string()
                })?;
            }
            _ => {
                warn!("Unsupported clock ID: {:?}", id);
                return Err(ErrorBase::InvalidArg { 
                    name: "clock_id", 
                    val: "unsupported".to_string() 
                });
            }
        }
        Ok(())
    }
}