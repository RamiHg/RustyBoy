use bitfield::bitfield;
use num_traits::FromPrimitive;

use crate::io_registers::Addresses;
use crate::mmu;
use crate::system::Interrupts;
//use crate::util::is_bit_set;

pub enum Key {
    Right,
    Left,
    Up,
    Down,
    Select,
    Start,
    B,
    A,
    NumKeys,
}

#[derive(Default)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Joypad {
    keys_pressed: [bool; Key::NumKeys as usize],
    ctrl: PadControl,
}

bitfield! {
    struct PadControl(i32);
    no default BitRange;
    impl Debug;
    u8;
    pub right_a, set_right_a: 0;
    pub left_b, set_left_b: 1;
    pub up_select, set_up_select: 2;
    pub down_start, set_down_start: 3;
    pub use_left, set_use_left: 4;
    pub use_right, set_use_right: 5;
}

define_typed_register!(PadControl, Addresses::Joypad);

impl Joypad {
    pub fn execute_tcycle(&mut self) -> Interrupts {
        // let new_reg_value = self.reg_value();
        // let current = self.reg_value;

        // let get_bit =
        //     |x| is_bit_set(x, 0) && is_bit_set(x, 1) && is_bit_set(x, 2) && is_bit_set(x, 3);

        // let new_bit = get_bit(new_reg_value);
        // let current_bit = get_bit(current);
        // let interrupts = if !new_bit && current_bit {
        //     Interrupts::JOYPAD
        // } else {
        //     Interrupts::empty()
        // };

        // self.reg_value = new_reg_value;
        // interrupts
        Interrupts::empty()
    }

    fn reg_value(&self) -> PadControl {
        use Key::*;
        let mut left = PadControl(0);
        if !self.ctrl.use_left() {
            left.set_right_a(self.keys_pressed[Right as usize]);
            left.set_left_b(self.keys_pressed[Left as usize]);
            left.set_up_select(self.keys_pressed[Up as usize]);
            left.set_down_start(self.keys_pressed[Down as usize]);
        }
        if !self.ctrl.use_right() {
            let mut ctrl2 = PadControl(0);
            ctrl2.set_right_a(self.keys_pressed[A as usize]);
            ctrl2.set_left_b(self.keys_pressed[B as usize]);
            ctrl2.set_up_select(self.keys_pressed[Select as usize]);
            ctrl2.set_down_start(self.keys_pressed[Start as usize]);
            left.0 |= ctrl2.0;
        }

        let mut ctrl = self.ctrl;
        ctrl.set_right_a(!left.right_a());
        ctrl.set_left_b(!left.left_b());
        ctrl.set_up_select(!left.up_select());
        ctrl.set_down_start(!left.down_start());
        ctrl
    }

    pub fn press(&mut self, key: Key) {
        self.keys_pressed[key as usize] = true;
    }

    pub fn release(&mut self, key: Key) {
        self.keys_pressed[key as usize] = false;
    }
}

impl mmu::MemoryMapped for Joypad {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(_, raw) = address;
        match Addresses::from_i32(raw) {
            Some(Addresses::Joypad) => Some(self.reg_value().0 | 0xC0),
            _ => None,
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw) = address;
        match Addresses::from_i32(raw) {
            Some(Addresses::Joypad) => {
                let value = PadControl(value);
                self.ctrl.set_use_left(value.use_left());
                self.ctrl.set_use_right(value.use_right());
                Some(())
            }
            _ => None,
        }
    }
}
