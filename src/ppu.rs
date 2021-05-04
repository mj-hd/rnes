use anyhow::Result;
use bitfield::bitfield;
use image::{ImageBuffer, Rgba};

use crate::bus::PpuBus;

const VISIBLE_WIDTH: usize = 256;
const VISIBLE_HEIGHT: usize = 240;
const WIDTH: usize = 340;
const HEIGHT: usize = 261;

bitfield! {
    struct Ctrl(u8);
    ie_nmi, _: 7;
    master, _: 6;
    large_sprite, _: 5;
    bg_pattern_table, _: 4;
    oam_pattern_table, _: 3;
    addr_inc_32, _: 2;
    name_table, _: 1, 0;
}

bitfield! {
    struct Mask(u8);
    blue, _: 7;
    green, _: 6;
    red, _: 5;
    oam, _: 4;
    bg, _: 3;
    oam_clip, _: 2;
    bg_clip, _: 1;
    mono, _: 0;
}

bitfield! {
    struct Status(u8);
    irq_vblank, _: 7;
    oam_0_hit, _: 6;
    oam_overflow, _: 5;
}

pub struct Ppu {
    bus: PpuBus,

    ctrl: Ctrl,
    mask: Mask,
    status: Status,

    dma_addr: u16,
    oam_addr: u8,
    buffer: Vec<u8>,

    scroll_x: u8,
    scroll_y: u8,

    pixels: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

impl Ppu {
    pub fn new(bus: PpuBus) -> Self {
        Self {
            bus,

            ctrl: Ctrl(0),
            mask: Mask(0),
            status: Status(0),

            oam_addr: 0,
            dma_addr: 0,
            buffer: Vec::with_capacity(2),

            scroll_x: 0,
            scroll_y: 0,

            pixels: ImageBuffer::new(VISIBLE_WIDTH as u32, VISIBLE_HEIGHT as u32),
        }
    }

    pub fn tick(&mut self) -> Result<()> {
        // TODO @mj-hd ここを実装する
        Ok(())
    }

    pub fn render(&mut self) -> Result<Vec<u8>> {
        Ok(self.pixels.clone().into_raw())
    }

    pub fn read_ctrl(&self) -> Result<u8> {
        Ok(self.ctrl.0)
    }

    pub fn read_mask(&self) -> Result<u8> {
        Ok(self.mask.0)
    }

    pub fn read_status(&mut self) -> Result<u8> {
        self.buffer.clear();

        Ok(self.status.0)
    }

    fn buffer_addr(&self) -> u16 {
        if self.buffer.len() != 2 {
            return 0;
        }

        self.buffer[1] as u16 | ((self.buffer[0] as u16) << 8)
    }

    fn set_buffer_addr(&mut self, addr: u16) {
        self.buffer.clear();
        self.buffer.push((addr >> 8) as u8);
        self.buffer.push((addr & 0xFF) as u8);
    }

    pub fn read_oam_data(&self) -> Result<u8> {
        // TODO OAM定義と実装
        Ok(0)
    }

    pub fn read_vram_data(&mut self) -> Result<u8> {
        let addr = self.buffer_addr();
        let result = self.bus.read(addr)?;

        self.set_buffer_addr(addr + if self.ctrl.addr_inc_32() { 32 } else { 1 });

        Ok(result)
    }

    pub fn read_oam_dma(&self) -> Result<u8> {
        Ok(self.oam_addr)
    }

    fn write_buffer(&mut self, data: u8) -> Result<()> {
        if self.buffer.len() >= 2 {
            self.buffer.clear();
        }

        self.buffer.push(data);

        Ok(())
    }

    pub fn write_ctrl(&mut self, data: u8) -> Result<()> {
        self.ctrl = Ctrl(data);

        Ok(())
    }

    pub fn write_mask(&mut self, data: u8) -> Result<()> {
        self.mask = Mask(data);

        Ok(())
    }

    pub fn write_status(&mut self, data: u8) -> Result<()> {
        self.status = Status(data);

        Ok(())
    }

    pub fn write_oam_addr(&mut self, data: u8) -> Result<()> {
        self.oam_addr = data;

        Ok(())
    }

    pub fn write_oam_data(&mut self, data: u8) -> Result<()> {
        // TODO OAM定義書き込み

        Ok(())
    }

    pub fn write_scroll(&mut self, data: u8) -> Result<()> {
        self.write_buffer(data)?;

        if self.buffer.len() == 2 {
            self.scroll_x = self.buffer[0];
            self.scroll_y = self.buffer[1];
        }

        Ok(())
    }

    pub fn write_vram_addr(&mut self, data: u8) -> Result<()> {
        self.write_buffer(data)
    }

    pub fn write_vram_data(&mut self, data: u8) -> Result<()> {
        let addr = self.buffer_addr();
        self.bus.write(addr, data)?;

        self.set_buffer_addr(addr + if self.ctrl.addr_inc_32() { 32 } else { 1 });

        Ok(())
    }

    pub fn write_oam_dma(&mut self, data: u8) -> Result<()> {
        self.dma_addr = (data as u16) << 8;

        self.bus.request_dma(self.dma_addr, self.oam_addr)?;

        Ok(())
    }
}
