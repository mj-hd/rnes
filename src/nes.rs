use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use crate::{apu::Apu, cpu::Cpu, mmc::new_mmc, ppu::Ppu, rom::Rom};

pub struct Nes {
    cpu: Cpu,
    ppu: Rc<RefCell<Ppu>>,
    apu: Rc<RefCell<Apu>>,
}

impl Nes {
    pub fn new(rom: Rom) -> Result<Self> {
        let mmc = Rc::new(RefCell::new(new_mmc(rom)?));
        let apu = Rc::new(RefCell::new(Apu::new()));
        let ppu = Rc::new(RefCell::new(Ppu::new(Rc::clone(&mmc))));
        let cpu = Cpu::new(Rc::clone(&mmc), Rc::clone(&ppu), Rc::clone(&apu));
        Ok(Self { cpu, ppu, apu })
    }

    pub fn tick(&mut self) -> Result<()> {
        Ok(())
    }
}
