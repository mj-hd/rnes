use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use crate::{apu::Apu, bus::CpuBus, mmc::Mmc, ppu::Ppu};

pub struct Cpu {
  bus: CpuBus,
}

impl Cpu {
  pub fn new(mmc: Rc<RefCell<Box<dyn Mmc>>>, ppu: Rc<RefCell<Ppu>>, apu: Rc<RefCell<Apu>>) -> Self {
    let bus = CpuBus::new(mmc, ppu, apu);

    Self { bus }
  }

  pub fn tick(&mut self) -> Result<()> {
    Ok(())
  }
}
