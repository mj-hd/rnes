use std::{cell::RefCell, rc::Rc, sync::mpsc::channel};

use anyhow::Result;

use crate::{
    apu::Apu,
    bus::{CpuBus, CpuBusEvent, PpuBus, PpuBusEvent},
    cpu::Cpu,
    mmc::new_mmc,
    ppu::Ppu,
    rom::Rom,
};

pub struct Nes {
    cpu: Rc<RefCell<Cpu>>,
    ppu: Rc<RefCell<Ppu>>,
    apu: Rc<RefCell<Apu>>,
}

impl Nes {
    pub fn new(rom: Rom) -> Result<Self> {
        let mmc = Rc::new(RefCell::new(new_mmc(rom)?));
        let apu = Rc::new(RefCell::new(Apu::new()));

        let (ppu_bus_sender, ppu_bus_event) = channel::<PpuBusEvent>();
        let (cpu_bus_sender, cpu_bus_event) = channel::<CpuBusEvent>();

        let ppu_bus = PpuBus::new(Rc::clone(&mmc), ppu_bus_event, cpu_bus_sender);
        let ppu = Rc::new(RefCell::new(Ppu::new(ppu_bus)));

        let cpu_bus = CpuBus::new(
            Rc::clone(&mmc),
            Rc::clone(&ppu),
            Rc::clone(&apu),
            cpu_bus_event,
            ppu_bus_sender,
        );
        let cpu = Rc::new(RefCell::new(Cpu::new(cpu_bus)));

        Ok(Self { cpu, ppu, apu })
    }

    pub fn tick(&mut self) -> Result<()> {
        self.cpu.borrow_mut().tick()?;
        self.ppu.borrow_mut().tick()?;

        Ok(())
    }

    pub fn render(&mut self) -> Result<Vec<u8>> {
        self.ppu.borrow_mut().render()
    }
}
