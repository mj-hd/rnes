use anyhow::Result;
use bitfield::bitfield;
use bitmatch::bitmatch;
use image::{ImageBuffer, Rgba};
use log::{debug, trace};

use crate::bus::PpuBus;

const VISIBLE_WIDTH: usize = 256;
const VISIBLE_HEIGHT: usize = 240;
const WIDTH: usize = 340;
const HEIGHT: usize = 261;

const COLORS: [[u8; 4]; 64] = [
    [0x80, 0x80, 0x80, 0xFF],
    [0x00, 0x3D, 0xA6, 0xFF],
    [0x00, 0x12, 0xB0, 0xFF],
    [0x44, 0x00, 0x96, 0xFF],
    [0xA1, 0x00, 0x5E, 0xFF],
    [0xC7, 0x00, 0x28, 0xFF],
    [0xBA, 0x06, 0x00, 0xFF],
    [0x8C, 0x17, 0x00, 0xFF],
    [0x5C, 0x2F, 0x00, 0xFF],
    [0x10, 0x45, 0x00, 0xFF],
    [0x05, 0x4A, 0x00, 0xFF],
    [0x00, 0x47, 0x2E, 0xFF],
    [0x00, 0x41, 0x66, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0x05, 0x05, 0x05, 0xFF],
    [0x05, 0x05, 0x05, 0xFF],
    [0xC7, 0xC7, 0xC7, 0xFF],
    [0x00, 0x77, 0xFF, 0xFF],
    [0x21, 0x55, 0xFF, 0xFF],
    [0x82, 0x37, 0xFA, 0xFF],
    [0xEB, 0x2F, 0xB5, 0xFF],
    [0xFF, 0x29, 0x50, 0xFF],
    [0xFF, 0x22, 0x00, 0xFF],
    [0xD6, 0x32, 0x00, 0xFF],
    [0xC4, 0x62, 0x00, 0xFF],
    [0x35, 0x80, 0x00, 0xFF],
    [0x05, 0x8F, 0x00, 0xFF],
    [0x00, 0x8A, 0x55, 0xFF],
    [0x00, 0x99, 0xCC, 0xFF],
    [0x21, 0x21, 0x21, 0xFF],
    [0x09, 0x09, 0x09, 0xFF],
    [0x09, 0x09, 0x09, 0xFF],
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0x0F, 0xD7, 0xFF, 0xFF],
    [0x69, 0xA2, 0xFF, 0xFF],
    [0xD4, 0x80, 0xFF, 0xFF],
    [0xFF, 0x45, 0xF3, 0xFF],
    [0xFF, 0x61, 0x8B, 0xFF],
    [0xFF, 0x88, 0x33, 0xFF],
    [0xFF, 0x9C, 0x12, 0xFF],
    [0xFA, 0xBC, 0x20, 0xFF],
    [0x9F, 0xE3, 0x0E, 0xFF],
    [0x2B, 0xF0, 0x35, 0xFF],
    [0x0C, 0xF0, 0xA4, 0xFF],
    [0x05, 0xFB, 0xFF, 0xFF],
    [0x5E, 0x5E, 0x5E, 0xFF],
    [0x0D, 0x0D, 0x0D, 0xFF],
    [0x0D, 0x0D, 0x0D, 0xFF],
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0xA6, 0xFC, 0xFF, 0xFF],
    [0xB3, 0xEC, 0xFF, 0xFF],
    [0xDA, 0xAB, 0xEB, 0xFF],
    [0xFF, 0xA8, 0xF9, 0xFF],
    [0xFF, 0xAB, 0xB3, 0xFF],
    [0xFF, 0xD2, 0xB0, 0xFF],
    [0xFF, 0xEF, 0xA6, 0xFF],
    [0xFF, 0xF7, 0x9C, 0xFF],
    [0xD7, 0xE8, 0x95, 0xFF],
    [0xA6, 0xED, 0xAF, 0xFF],
    [0xA2, 0xF2, 0xDA, 0xFF],
    [0x99, 0xFF, 0xFC, 0xFF],
    [0xDD, 0xDD, 0xDD, 0xFF],
    [0x11, 0x11, 0x11, 0xFF],
    [0x11, 0x11, 0x11, 0xFF],
];

#[derive(Debug, Clone, Copy)]
struct Color {
    value: usize,
    transparent: bool,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            value: 0,
            transparent: true,
        }
    }
}

impl Color {
    fn to_pixel(self) -> Rgba<u8> {
        Rgba(COLORS[self.value])
    }
}

#[derive(Debug, Clone, Copy)]
struct OamColor {
    color: Color,
    behind: bool,
    zero: bool,
}

impl Default for OamColor {
    fn default() -> Self {
        Self {
            color: Default::default(),
            behind: false,
            zero: false,
        }
    }
}

type ColorIndex = usize;

#[derive(Debug, PartialEq)]
enum Mode {
    Idle,
    Drawing,
    OamScan,
    PostIdle,
    VBlank,
}

bitfield! {
    #[derive(Default, Copy, Clone)]
    struct SpriteFlags(u8);
    impl Debug;
    palette_num, _: 1, 0;
    priority, _: 5;
    x_flip, _: 6;
    y_flip, _: 7;
}

#[derive(Debug, Default, Copy, Clone)]
struct Oam {
    y: u8,
    x: u8,
    tile_num: u8,
    sprite_flag: SpriteFlags,
    zero: bool,
}

impl Oam {
    fn new(data: &[u8], zero: bool) -> Self {
        Oam {
            y: data[0],
            x: data[3],
            tile_num: data[1],
            sprite_flag: SpriteFlags(data[2]),
            zero,
        }
    }

    #[bitmatch]
    fn large_tile_base_addr(&self) -> u16 {
        #[bitmatch]
        let "tttttttb" = self.tile_num;

        let base_addr = if b == 1 { 0x1000u16 } else { 0x0000u16 };
        base_addr + t as u16
    }

    fn tile(&self, row: u8) -> u8 {
        if row >= 8 {
            self.tile_num + 1
        } else {
            self.tile_num
        }
    }
}

bitfield! {
    #[derive(Clone, Copy)]
    struct Ctrl(u8);
    impl Debug;
    ie_nmi, _: 7;
    master, _: 6;
    large_sprite, _: 5;
    bg_pattern_table, _: 4;
    oam_pattern_table, _: 3;
    addr_inc_32, _: 2;
    name_table, _: 1, 0;
}

bitfield! {
    #[derive(Clone, Copy)]
    struct Mask(u8);
    impl Debug;
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
    #[derive(Clone, Copy)]
    struct Status(u8);
    impl Debug;
    irq_vblank, set_irq_vblank: 7;
    oam_0_hit, set_oam_0_hit: 6;
    oam_overflow, set_oam_overflow: 5;
}

bitfield! {
    #[derive(Clone, Copy)]
    struct Attribute(u8);
    impl Debug;
    u8, palette, _: 1, 0, 4;
}

impl Attribute {
    pub fn index_for(&self, tile_x: u8, tile_y: u8) -> u8 {
        let x = tile_x / 2;
        let y = tile_y / 2;
        let x_index = (x + 1) % 2;
        let y_index = (y + 1) % 2;
        self.palette((3 - x_index - y_index * 2) as usize)
    }
}

pub struct Ppu {
    bus: PpuBus,

    ctrl: Ctrl,
    mask: Mask,
    status: Status,

    dma_addr: u16,
    oam_addr: u8,
    buffer: Vec<u8>,
    mode: Mode,

    x: u8,
    y: u8,
    scroll_x: u8,
    scroll_y: u8,

    cycles: usize,
    lines: usize,

    cur_bg: [Color; 8],

    bg_line: [Color; WIDTH],
    oam_line: [OamColor; WIDTH],

    pixels: ImageBuffer<Rgba<u8>, Vec<u8>>,

    pub nmi: bool,
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
            mode: Mode::Idle,

            x: 0,
            y: 0,
            scroll_x: 0,
            scroll_y: 0,

            cycles: 0,
            lines: 0,

            cur_bg: [Default::default(); 8],
            bg_line: [Default::default(); WIDTH],
            oam_line: [Default::default(); WIDTH],

            pixels: ImageBuffer::new(VISIBLE_WIDTH as u32, VISIBLE_HEIGHT as u32),

            nmi: false,
        }
    }

    pub fn tick(&mut self) -> Result<()> {
        self.cycles += 1;

        self.bus.tick()?;

        if self.cycles == WIDTH {
            self.cycles = 0;
            self.lines += 1;
        }

        if self.cycles == 0 {
            if self.lines == HEIGHT {
                self.lines = 0;
                self.status.set_irq_vblank(false);
                self.nmi = false;
            }

            if self.lines == VISIBLE_HEIGHT {
                self.y = 0;
                self.mode = Mode::VBlank;
                self.status.set_irq_vblank(true);

                if self.ctrl.ie_nmi() {
                    self.nmi = true;
                }
            }
        }

        if self.lines < VISIBLE_HEIGHT {
            self.y = self.lines as u8;

            match self.cycles {
                0 => {
                    self.x = 0;
                    self.mode = Mode::Idle;
                }
                1..=256 => {
                    self.x = (self.cycles - 1) as u8;
                    self.mode = Mode::Drawing;
                }
                257..=320 => {
                    self.mode = Mode::OamScan;
                }
                321..=340 => {
                    self.mode = Mode::PostIdle;
                }
                _ => {}
            }
        }

        match self.mode {
            Mode::Drawing => {
                self.draw_bg()?;

                self.put_pixels()?;
            }
            Mode::OamScan => {
                self.draw_sprites(self.cycles % 64)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn draw_bg(&mut self) -> Result<()> {
        if !self.mask.bg() {
            return Ok(());
        }

        let cx = self.x.wrapping_add(self.scroll_x);
        let cy = self.y.wrapping_add(self.scroll_y);
        let col = cx % 8;
        let row = cy % 8;
        let tile_x = cx / 8;
        let tile_y = cy / 8;

        if col == 0 {
            let attr = self.bg_attr(tile_x, tile_y)?;
            let tile = self.bg_tile(tile_x, tile_y)?;
            let indexes = self.to_indexes(tile, row, self.bg_pattern_table_addr())?;
            let palettes = self.bg_palettes(tile_x, tile_y, attr)?;

            self.cur_bg = self.to_colors(indexes, palettes);
        }

        self.bg_line[self.x as usize] = self.cur_bg[col as usize];

        Ok(())
    }

    fn draw_sprites(&mut self, i: usize) -> Result<()> {
        if !self.mask.oam() {
            return Ok(());
        }

        let size = if self.ctrl.large_sprite() { 16 } else { 8 };

        let oam = Oam::new(&self.bus.oam[(i * 4)..((i + 1) * 4)], i == 0);
        let cur_y = self.lines as u16;
        let target_y = oam.y as u16;

        if cur_y < target_y + size && target_y <= cur_y {
            self.draw_sprite(oam)?;
        }

        Ok(())
    }

    fn draw_sprite(&mut self, oam: Oam) -> Result<()> {
        let size = if self.ctrl.large_sprite() { 16 } else { 8 };

        let row = if oam.sprite_flag.y_flip() {
            size - (self.y - oam.y)
        } else {
            self.y - oam.y
        };

        let tile = oam.tile(row);

        let base_addr = if self.ctrl.large_sprite() {
            oam.large_tile_base_addr()
        } else {
            self.oam_pattern_table_addr()
        };

        let indexes = self.to_indexes(tile, row, base_addr)?;
        let palette_num = oam.sprite_flag.palette_num();
        let palettes = self.sprite_palettes(palette_num)?;

        let colors = self.to_colors(indexes, palettes);

        let cx = oam.x as usize;

        let mut oam_colors = [Default::default(); 8];

        for (i, color) in colors.iter().enumerate() {
            let i = if oam.sprite_flag.x_flip() { 7 - i } else { i };

            oam_colors[i] = OamColor {
                color: *color,
                behind: oam.sprite_flag.priority(),
                zero: oam.zero,
            };
        }

        self.oam_line[cx..(cx + 8)].copy_from_slice(&oam_colors[..]);

        Ok(())
    }

    fn name_table_addr(&self) -> u16 {
        match self.ctrl.name_table() {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => 0,
        }
    }

    fn bg_pattern_table_addr(&self) -> u16 {
        match self.ctrl.bg_pattern_table() {
            false => 0x0000,
            true => 0x1000,
        }
    }

    fn oam_pattern_table_addr(&self) -> u16 {
        match self.ctrl.oam_pattern_table() {
            false => 0x0000,
            true => 0x1000,
        }
    }

    fn bg_attr(&self, tile_x: u8, tile_y: u8) -> Result<Attribute> {
        let attr_x = tile_x / 4;
        let attr_y = tile_y / 4;
        let base_addr = self.name_table_addr() + 0x03C0;
        let index_addr = attr_x as u16 + (attr_y as u16) * 8;
        let addr = base_addr.wrapping_add(index_addr as u16);

        let attr = Attribute(self.bus.read(addr)?);

        Ok(attr)
    }

    fn bg_tile(&self, tile_x: u8, tile_y: u8) -> Result<u8> {
        let base_addr = self.name_table_addr();
        let index_addr = tile_x as u16 + (tile_y as u16) * 32;
        let addr = base_addr.wrapping_add(index_addr as u16);

        self.bus.read(addr)
    }

    #[bitmatch]
    #[allow(clippy::many_single_char_names)]
    fn to_indexes(&self, tile: u8, row: u8, base_addr: u16) -> Result<[ColorIndex; 8]> {
        let addr = base_addr + row as u16 + (tile as u16) * 16;

        let bit = self.bus.read(addr)?;
        let color = self.bus.read(addr + 8)?;

        let mut indexes = [0; 8];

        #[bitmatch]
        let "acegikmo" = color;

        #[bitmatch]
        let "bdfhjlnp" = bit;

        #[bitmatch]
        let "aabbccddeeffgghh" = bitpack!("abcdefghijklmnop");

        for (j, &index) in [a, b, c, d, e, f, g, h].iter().enumerate() {
            indexes[j] = index as usize;
        }

        Ok(indexes)
    }

    fn bg_palettes(&self, tile_x: u8, tile_y: u8, attr: Attribute) -> Result<[Color; 4]> {
        let base_addr = 0x3F00u16;
        let palette_index = attr.index_for(tile_x, tile_y);
        let index_addr = palette_index * 0x04;
        let addr = base_addr + index_addr as u16;

        let mut palettes: [Color; 4] = [Default::default(); 4];

        for i in 0..4 {
            palettes[i] = Color {
                value: self.bus.read(addr + i as u16)? as usize,
                transparent: i == 0,
            };
        }

        Ok(palettes)
    }

    fn sprite_palettes(&self, palette_num: u8) -> Result<[Color; 4]> {
        let base_addr = 0x3F10u16;
        let index_addr = palette_num * 0x04;
        let addr = base_addr + index_addr as u16;

        let mut palettes: [Color; 4] = [Default::default(); 4];

        for i in 0..4 {
            palettes[i] = Color {
                value: self.bus.read(addr + i as u16)? as usize,
                transparent: i == 0,
            };
        }

        Ok(palettes)
    }

    fn to_colors(&self, indexes: [ColorIndex; 8], palettes: [Color; 4]) -> [Color; 8] {
        let mut colors: [Color; 8] = [Default::default(); 8];

        for i in 0..8 {
            colors[i] = palettes[indexes[i]];
        }

        colors
    }

    fn put_pixels(&mut self) -> Result<()> {
        let backdrop = self.bus.read(0x3F00)? as usize;
        let mut pixel = Rgba(COLORS[backdrop]);

        let bg_color = self.bg_line[self.x as usize];
        let sprite_color = self.oam_line[self.x as usize];

        if self.mask.bg() && !bg_color.transparent {
            pixel = bg_color.to_pixel();
        }

        if self.mask.oam() {
            if sprite_color.behind {
                if self.mask.bg() || bg_color.transparent {
                    pixel = sprite_color.color.to_pixel();
                }
            } else {
                if !sprite_color.color.transparent {
                    pixel = sprite_color.color.to_pixel();
                }
            }
        }

        if self.mask.bg() && self.mask.oam() {
            if sprite_color.zero && bg_color.transparent && sprite_color.color.transparent {
                self.status.set_oam_0_hit(true);
            }
        }

        self.pixels.put_pixel(self.x as u32, self.y as u32, pixel);

        self.bg_line[self.x as usize] = Default::default();
        self.oam_line[self.x as usize] = Default::default();

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

        let status = self.status.clone();

        self.status.set_irq_vblank(false);
        self.status.set_oam_0_hit(false);
        self.status.set_oam_overflow(false);

        Ok(status.0)
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
        let ctrl = Ctrl(data);

        if !self.ctrl.ie_nmi() && ctrl.ie_nmi() && self.mode == Mode::VBlank {
            self.nmi = true;
        }

        self.ctrl = ctrl;

        Ok(())
    }

    pub fn write_mask(&mut self, data: u8) -> Result<()> {
        self.mask = Mask(data);

        debug!("WRITE MASK: {:?}", self.mask);

        Ok(())
    }

    pub fn write_status(&mut self, data: u8) -> Result<()> {
        self.status = Status(data);

        Ok(())
    }

    pub fn write_oam_addr(&mut self, data: u8) -> Result<()> {
        self.oam_addr = data;

        trace!("WRITE OAM ADDR: {:#02X}", data);

        Ok(())
    }

    pub fn write_oam_data(&mut self, data: u8) -> Result<()> {
        self.bus.oam[self.oam_addr as usize] = data;

        trace!("WRITE OAM: {:#04X} = {:#02X}", self.oam_addr, data);

        Ok(())
    }

    pub fn write_scroll(&mut self, data: u8) -> Result<()> {
        self.write_buffer(data)?;

        if self.buffer.len() == 2 {
            self.scroll_x = self.buffer[0];
            self.scroll_y = self.buffer[1];
        }

        trace!(
            "WRITE SCROLL: {} ({},{})",
            data,
            self.scroll_x,
            self.scroll_y
        );

        Ok(())
    }

    pub fn write_vram_addr(&mut self, data: u8) -> Result<()> {
        self.write_buffer(data)
    }

    pub fn write_vram_data(&mut self, data: u8) -> Result<()> {
        let addr = self.buffer_addr();
        self.bus.write(addr, data)?;

        debug!("WRITE VRAM: {:#04X} = {:#02X}", addr, data);

        self.set_buffer_addr(addr + if self.ctrl.addr_inc_32() { 32 } else { 1 });

        Ok(())
    }

    pub fn write_oam_dma(&mut self, data: u8) -> Result<()> {
        self.dma_addr = (data as u16) << 8;

        self.bus.request_dma(self.dma_addr, self.oam_addr)?;

        debug!(
            "REQUEST DMA: {:#04X} -> {:#04X}",
            self.dma_addr, self.oam_addr
        );

        Ok(())
    }
}
