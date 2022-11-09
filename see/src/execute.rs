use crate::{trap_vec, Supervisor};
use riscv::register::*;

pub(crate) fn execute_supervisor(supervisor: Supervisor) {
    use core::arch::asm;

    unsafe {
        mstatus::set_mpp(mstatus::MPP::Supervisor);
        mstatus::set_mie();
    };

    let mut ctx = Context::new(supervisor);

    unsafe {
        asm!("csrw     mip, {}", in(reg) 0);
        asm!("csrw mideleg, {}", in(reg) usize::MAX);
        mstatus::clear_mie();
        medeleg::set_load_page_fault();
        medeleg::set_store_page_fault();
        medeleg::set_user_env_call();
        trap_vec::load(true);
        mie::set_mext();
        mie::set_msoft();
        mie::set_mtimer();
    }

    loop {
        use mcause::{Exception as E, Trap as T};

        unsafe { trap_vec::m_to_s(&mut ctx) };

        match mcause::read().cause() {
            T::Exception(E::SupervisorEnvCall) => {
                if !ctx.handle_ecall() {
                    return;
                }
            }
            T::Exception(E::IllegalInstruction) => {
                let ins = mtval::read();
                if !ctx.emulate_rdtime(ins) {
                    ctx.trap_stop(T::Exception(E::IllegalInstruction));
                }
            }
            trap => ctx.trap_stop(trap),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct Context {
    msp: usize,
    x: [usize; 31],
    mstatus: usize,
    mepc: usize,
}

impl Context {
    fn new(supervisor: Supervisor) -> Self {
        let mut ctx = Self {
            msp: 0,
            x: [0; 31],
            mstatus: 0,
            mepc: supervisor.start_addr,
        };

        unsafe { core::arch::asm!("csrr {}, mstatus", out(reg) ctx.mstatus) };
        *ctx.a_mut(0) = 0;
        *ctx.a_mut(1) = supervisor.opaque;

        ctx
    }

    #[inline]
    fn x(&self, n: usize) -> usize {
        self.x[n - 1]
    }

    #[inline]
    fn x_mut(&mut self, n: usize) -> &mut usize {
        &mut self.x[n - 1]
    }

    #[inline]
    fn a(&self, n: usize) -> usize {
        self.x(n + 10)
    }

    #[inline]
    fn a_mut(&mut self, n: usize) -> &mut usize {
        self.x_mut(n + 10)
    }

    fn handle_ecall(&mut self) -> bool {
        use rustsbi::spec::{binary::*, hsm::*, srst::*};
        let extension = self.a(7);
        let function = self.a(6);
        let ans = crate::extensions::sbi().handle_ecall(
            extension,
            function,
            [
                self.a(0),
                self.a(1),
                self.a(2),
                self.a(3),
                self.a(4),
                self.a(5),
            ],
        );
        // 判断导致退出执行流程的调用
        if ans.error == RET_SUCCESS {
            match extension {
                // 核状态
                EID_HSM => match function {
                    HART_STOP => return false,
                    HART_SUSPEND
                        if matches!(
                            u32::try_from(self.a(0)),
                            Ok(HART_SUSPEND_TYPE_NON_RETENTIVE)
                        ) =>
                    {
                        return false;
                    }
                    _ => {}
                },
                // 系统重置
                EID_SRST => match function {
                    SYSTEM_RESET
                        if matches!(
                            u32::try_from(self.a(0)),
                            Ok(RESET_TYPE_COLD_REBOOT) | Ok(RESET_TYPE_WARM_REBOOT)
                        ) =>
                    {
                        return false;
                    }
                    _ => {}
                },

                _ => {}
            }
        }
        *self.a_mut(0) = ans.error;
        *self.a_mut(1) = ans.value;
        self.mepc = self.mepc.wrapping_add(4);
        true
    }

    fn emulate_rdtime(&mut self, ins: usize) -> bool {
        const RD_MASK: usize = ((1 << 5) - 1) << 7;
        if ins & !RD_MASK == 0xC0102073 {
            // rdtime is actually a csrrw instruction

            let rd = (ins & RD_MASK) >> RD_MASK.trailing_zeros();
            if rd != 0 {
                *self.x_mut(rd) = time::read();
            }

            self.mepc = self.mepc.wrapping_add(4); // skip current instruction
            true
        } else {
            false // is not a rdtime instruction
        }
    }

    fn trap_stop(&self, trap: mcause::Trap) -> ! {
        println!(
            "
-----------------------------
> exception: {trap:?}
> mstatus:   {:#018x}
> mepc:      {:#018x}
> mtval:     {:#018x}
-----------------------------
",
            self.mstatus,
            self.mepc,
            mtval::read()
        );
        loop {
            core::hint::spin_loop();
        }
    }
}
