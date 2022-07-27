use core::fmt::{Arguments, Result, Write};
use hal::pac::UART0;

struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> Result {
        let uart = unsafe { &*UART0::ptr() };
        for ch in s.bytes() {
            // 等待 FIFO 空位
            while uart.usr.read().tfnf().is_full() {
                core::hint::spin_loop();
            }
            uart.thr().write(|w| w.thr().variant(ch));
        }
        Ok(())
    }
}

#[inline]
pub fn print(args: Arguments) {
    Console.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::print(core::format_args!($($arg)*));
    }
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {{
        $crate::console::print(core::format_args!($($arg)*));
        $crate::print!("\n");
    }}
}
