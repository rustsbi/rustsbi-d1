fn main() {
    use std::{env, fs, path::PathBuf};

    let ld = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("test-kernel.ld");
    fs::write(ld, LINKER).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOG");
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}

const LINKER: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(_start)
MEMORY {
    DDR : ORIGIN = 0x40200000, LENGTH = 16M
}
SECTIONS {
    .text : {
        *(.text.entry)
        *(.text .text.*)
    } > DDR
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } > DDR
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    } > DDR
    .bss (NOLOAD) : {
        *(.bss.uninit)
        . = ALIGN(8);
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        . = ALIGN(8);
        ebss = .;
    } > DDR
    /DISCARD/ : {
        *(.eh_frame)
    }
}";
