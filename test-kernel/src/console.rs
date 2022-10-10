use core::fmt::{Arguments, Result, Write};
use hal::pac::UART0;
use log::{Level, LevelFilter, Log};

pub(crate) fn init() {
    log::set_logger(&Console).unwrap();
    log::set_max_level(LevelFilter::Trace);
}

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

impl Log for Console {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let color_code = match record.level() {
                Level::Error => "31",
                Level::Warn => "93",
                Level::Info => "34",
                Level::Debug => "32",
                Level::Trace => "90",
            };
            println!(
                "\x1b[{}m[{:>5}] {}\x1b[0m",
                color_code,
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
