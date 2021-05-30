use anyhow::Result;
use log::debug;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive)]
pub enum JoypadKey {
    A = 0,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

impl JoypadKey {
    fn next(&self) -> Self {
        FromPrimitive::from_u8(((*self as u8) + 1) % (JoypadKey::Right as u8 + 1)).unwrap()
    }
}

pub struct Joypad {
    strobe: bool,

    cur_key: JoypadKey,

    state: HashMap<JoypadKey, bool>,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            strobe: false,
            cur_key: JoypadKey::A,
            state: HashMap::new(),
        }
    }

    pub fn read(&mut self) -> Result<u8> {
        let pressed = self.state.get(&self.cur_key).unwrap_or(&false);

        debug!("READ JOYPAD: {:?} {}", self.cur_key, pressed);

        if !self.strobe {
            self.cur_key = self.cur_key.next();
        }

        Ok(*pressed as u8)
    }

    pub fn write(&mut self, data: u8) -> Result<()> {
        self.strobe = data >> 7 == 1;

        debug!("WRITE JOYPAD: {:#02X}", data);

        if self.strobe {
            self.cur_key = JoypadKey::A;
        }

        Ok(())
    }

    pub fn keydown(&mut self, key: JoypadKey) {
        debug!("KEYDOWN JOYPAD: {:?}", key);

        self.state.insert(key, true);
    }

    pub fn keyup(&mut self, key: JoypadKey) {
        debug!("KEYUP JOYPAD: {:?}", key);

        self.state.insert(key, false);
    }
}
