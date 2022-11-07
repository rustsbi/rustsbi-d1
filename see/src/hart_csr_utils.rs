pub(crate) fn print_pmps() {
    const ITEM_PER_CFG: usize = core::mem::size_of::<usize>();
    const CFG_STEP: usize = ITEM_PER_CFG / core::mem::size_of::<u32>();

    let mut i_cfg = 0;
    while i_cfg < 4 {
        let base = i_cfg * core::mem::size_of::<u32>();
        let mut cfg = pmpcfg(i_cfg);
        for i_addr in 0..ITEM_PER_CFG {
            match (cfg >> 3) & 0b11 {
                0b00 => {}
                0b01 => dump_pmp(
                    base + i_addr,
                    pmpaddr(base + i_addr - 1) << 2,
                    pmpaddr(base + i_addr) << 2,
                    cfg,
                ),
                0b10 => {
                    let s = pmpaddr(base + i_addr);
                    dump_pmp(base + i_addr, s << 2, (s + 1) << 2, cfg);
                }
                0b11 => {
                    let addr = pmpaddr(base + i_addr);
                    let len = 1usize << (addr.trailing_ones() + 2);
                    let s = (addr & !(len - 1)) << 2;
                    let e = s + len;
                    dump_pmp(base + i_addr, s, e, cfg);
                }
                _ => unreachable!(),
            };
            cfg >>= 8;
        }
        i_cfg += CFG_STEP;
    }
}

#[inline]
fn dump_pmp(i: usize, s: usize, e: usize, cfg: usize) {
    println!(
        "[rustsbi] pmp{i:02}: {s:#010x}..{e:#010x} ({}{}{})",
        if cfg & 0b100 != 0 { "x" } else { "-" },
        if cfg & 0b010 != 0 { "w" } else { "-" },
        if cfg & 0b001 != 0 { "r" } else { "-" },
    );
}

fn pmpcfg(i: usize) -> usize {
    use riscv::register::*;
    match i {
        0 => pmpcfg0::read().bits,
        #[cfg(target_arch = "riscv32")]
        1 => pmpcfg1::read().bits,
        2 => pmpcfg2::read().bits,
        #[cfg(target_arch = "riscv32")]
        3 => pmpcfg3::read().bits,
        _ => todo!(),
    }
}

fn pmpaddr(i: usize) -> usize {
    use riscv::register::*;
    match i {
        0x0 => pmpaddr0::read(),
        0x1 => pmpaddr1::read(),
        0x2 => pmpaddr2::read(),
        0x3 => pmpaddr3::read(),
        0x4 => pmpaddr4::read(),
        0x5 => pmpaddr5::read(),
        0x6 => pmpaddr6::read(),
        0x7 => pmpaddr7::read(),
        0x8 => pmpaddr8::read(),
        0x9 => pmpaddr9::read(),
        0xa => pmpaddr10::read(),
        0xb => pmpaddr11::read(),
        0xc => pmpaddr12::read(),
        0xd => pmpaddr13::read(),
        0xe => pmpaddr14::read(),
        0xf => pmpaddr15::read(),
        _ => todo!(),
    }
}
