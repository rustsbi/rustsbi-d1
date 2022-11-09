fn main() {
    use std::{env, fs, path::PathBuf};

    let ld = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("see.ld");
    fs::write(ld, LINKER).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOG");
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}

const LINKER: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(_start)
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
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } > DDR
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    } > DDR
    sidata = LOADADDR(.data);
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
