fn main() {
    use std::{env, fs, path::PathBuf};

    let ld = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("see.ld");
    fs::write(ld, LINKER).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}

const LINKER: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(entry)
MEMORY {
    DDR : ORIGIN = 0x40000000, LENGTH = 2M
}
SECTIONS {
    .text : {
        *(.text.entry)
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
