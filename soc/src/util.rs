/// Hodgepodge of util classes. Mostly integer stuff. Timers used by audio. Atomic trait.
use num_traits::PrimInt;
use std::sync::atomic::Ordering;

macro_rules! strict_fail {
    ($($vals:expr),*) => {
        #[cfg(feature = "strict_assert")]
        panic!($($vals),*);
    }
}

macro_rules! strict_assert_lt {
    ($($vals:expr),*) => {
        #[cfg(feature = "strict_assert")]
        assert_lt!($($vals),*);
    };
}

macro_rules! strict_assert {
    ($($vals:expr),*) => {
        #[cfg(feature = "strict_assert")]
        assert!($($vals),*);
    };
}

pub fn is_8bit(value: i32) -> bool {
    (value as u32) <= core::u8::MAX.into()
}

pub fn is_16bit(value: i32) -> bool {
    (value as u32) <= core::u16::MAX.into()
}

pub fn is_bit_set(value: i32, bit: i32) -> bool {
    (value & (1 << bit)) != 0
}

pub fn upper_5_bits(value: i32) -> i32 {
    (value & 0xF8) >> 3
}

pub fn reverse_16bits(mut value: i32) -> i32 {
    let mut result = 0;
    for _ in 0..16 {
        result >>= 1;
        result |= value & 0x8000;
        value <<= 1;
    }
    result
}

/// Util timer class.
pub type Timer = std::iter::Rev<std::ops::Range<i32>>;
pub fn timer(count: i32) -> Timer {
    (0..count).rev()
}

#[derive(Clone, Debug)]
pub struct CountdownTimer {
    counter: i32,
    timer: std::iter::Cycle<Timer>,
}

impl Iterator for CountdownTimer {
    type Item = Option<i32>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.counter > 0 {
            Some(if self.timer.next().unwrap() == 0 {
                self.counter -= 1;
                Some(self.counter)
            } else {
                None
            })
        } else {
            None
        }
    }
}

impl CountdownTimer {
    #[allow(dead_code)]
    pub fn new(counter: i32, period: i32) -> CountdownTimer {
        CountdownTimer { counter, timer: timer(period).cycle() }
    }
}

/// Iterator that iterates over bits of an integer.
#[allow(dead_code)]
pub fn iterate_bits<T: PrimInt>(mut value: T) -> impl Iterator<Item = bool> {
    let mut bit = 0;
    std::iter::from_fn(move || {
        if bit < T::zero().count_zeros() {
            bit += 1;
            let result = !(value & T::one()).is_zero();
            value = value >> 1;
            Some(result)
        } else {
            None
        }
    })
}

/// Helpful trait to allow using atomics with any data type convertable from/to primitive integers.
/// Used mainly with the audio driver.
pub trait AtomicInt<T: PrimInt> {
    fn weak_update_with<U: From<T> + Into<T>>(
        &self,
        success: Ordering,
        with: impl Fn(U) -> U,
    ) -> U {
        let mut current = self.load_relaxed();
        loop {
            let new = with(current.into()).into();
            match self.compare_exchange_weak_relaxed(current, new, success) {
                Ok(_) => break current.into(),
                Err(x) => current = x,
            }
        }
    }
    fn load_relaxed(&self) -> T;
    fn compare_exchange_weak_relaxed(&self, current: T, new: T, ordering: Ordering)
        -> Result<T, T>;
}

macro_rules! impl_weak_atomic {
    ($tprim:ty, $tatomic:ty) => {
        impl AtomicInt<$tprim> for $tatomic {
            fn load_relaxed(&self) -> $tprim {
                self.load(Ordering::Relaxed)
            }
            fn compare_exchange_weak_relaxed(
                &self,
                current: $tprim,
                new: $tprim,
                ordering: Ordering,
            ) -> Result<$tprim, $tprim> {
                self.compare_exchange_weak(current, new, ordering, Ordering::Relaxed)
            }
        }
    };
}

impl_weak_atomic!(u8, std::sync::atomic::AtomicU8);
impl_weak_atomic!(u64, std::sync::atomic::AtomicU64);
