fn main() {
    use std::{env, fs, path::PathBuf};

    let ld = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("see.ld");
    fs::write(ld, NEZHA_FLASH).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}

const NEZHA_FLASH: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(head_jump)
MEMORY {
    DDR : ORIGIN = 0x40000000, LENGTH = 2M
}
SECTIONS {
    .head : {
        KEEP(*(.head.jump))
        KEEP(*(.head.info))
    } > DDR
    .text : ALIGN(4) {
        KEEP(*(.text.entry))
        . = ALIGN(4);
        *(.text.trap_handler)
        *(.text .text.*)
    } > DDR
    .rodata : ALIGN(8) {
        srodata = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        erodata = .;
    } > DDR
    .data : ALIGN(8) {
        sdata = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        edata = .;
    } > DDR
    sidata = LOADADDR(.data);
    .bss (NOLOAD) : ALIGN(8) {
        *(.bss.uninit)
        . = ALIGN(8);
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        ebss = .;
    } > DDR
    /DISCARD/ : {
        *(.eh_frame)
    }
}";
