use core::ops::Shl;
use hal::pac::UART0;

pub struct Out;

pub struct Endl;

pub enum Hex {
    Raw(usize),
    Fmt(usize),
}

impl Shl<u8> for Out {
    type Output = Self;

    #[inline]
    fn shl(self, rhs: u8) -> Self::Output {
        let uart = unsafe { &*UART0::ptr() };
        // 等待 FIFO 空位
        while uart.usr.read().tfnf().is_full() {
            core::hint::spin_loop();
        }
        uart.thr().write(|w| w.thr().variant(rhs));
        self
    }
}

impl Shl<&str> for Out {
    type Output = Self;

    #[inline]
    fn shl(mut self, rhs: &str) -> Self::Output {
        for c in rhs.bytes() {
            self = self << c;
        }
        self
    }
}

impl Shl<Endl> for Out {
    type Output = Self;

    #[inline]
    fn shl(self, _: Endl) -> Self::Output {
        self << "\r\n"
    }
}

impl Shl<usize> for Out {
    type Output = Self;

    #[inline]
    fn shl(mut self, mut rhs: usize) -> Self::Output {
        if rhs == 0 {
            self << b'0'
        } else {
            let mut bits = 1;
            while bits < rhs {
                bits *= 10;
            }
            bits /= 10;
            while bits > 0 {
                self = self << ((rhs / bits) as u8 + b'0');
                rhs %= bits;
                bits /= 10;
            }
            self
        }
    }
}

impl Shl<Hex> for Out {
    type Output = Self;

    fn shl(mut self, rhs: Hex) -> Self::Output {
        let num = match rhs {
            Hex::Raw(n) => n,
            Hex::Fmt(n) => {
                self = self << "0x";
                n
            }
        };
        if num == 0 {
            self << b'0'
        } else {
            (0..16)
                .rev()
                .map(|bits| ((num >> (bits * 4)) & 0xf) as u8)
                .skip_while(|x| *x == 0)
                .fold(self, |out, x| {
                    out << x + if x < 10 { b'0' } else { b'a' - 10 }
                })
        }
    }
}
