use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use crate::{bus::PpuBus, mmc::Mmc};

pub struct Ppu {
  bus: PpuBus,
}

impl Ppu {
  pub fn new(mmc: Rc<RefCell<Box<dyn Mmc>>>) -> Self {
    let bus = PpuBus::new(mmc);
    Self { bus }
  }

  pub fn read_control1(&self) -> Result<u8> {
    Ok(0)
  }

  pub fn read_control2(&self) -> Result<u8> {
    Ok(0)
  }

  pub fn read_status(&self) -> Result<u8> {
    Ok(0)
  }

  pub fn read_oam_addr(&self) -> Result<u8> {
    Ok(0)
  }

  pub fn read_oam_data(&self) -> Result<u8> {
    Ok(0)
  }

  pub fn read_scroll(&self) -> Result<u8> {
    Ok(0)
  }

  pub fn read_vram_addr(&self) -> Result<u8> {
    Ok(0)
  }

  pub fn read_vram_data(&self) -> Result<u8> {
    Ok(0)
  }

  pub fn read_oam_dma(&self) -> Result<u8> {
    Ok(0)
  }

  pub fn write_control1(&mut self, data: u8) -> Result<()> {
    Ok(())
  }

  pub fn write_control2(&mut self, data: u8) -> Result<()> {
    Ok(())
  }

  pub fn write_status(&mut self, data: u8) -> Result<()> {
    Ok(())
  }

  pub fn write_oam_addr(&mut self, data: u8) -> Result<()> {
    Ok(())
  }

  pub fn write_oam_data(&mut self, data: u8) -> Result<()> {
    Ok(())
  }

  pub fn write_scroll(&mut self, data: u8) -> Result<()> {
    Ok(())
  }

  pub fn write_vram_addr(&mut self, data: u8) -> Result<()> {
    Ok(())
  }

  pub fn write_vram_data(&mut self, data: u8) -> Result<()> {
    Ok(())
  }

  pub fn write_oam_dma(&mut self, data: u8) -> Result<()> {
    Ok(())
  }
}
