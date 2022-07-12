#[macro_use]
extern crate clap;

use clap::Parser;
use command_ext::{BinUtil, Cargo, CommandExt, Ext};
use once_cell::sync::OnceCell;
use std::{
    ffi::OsStr,
    fs::{write as file, File},
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[clap(name = "NeZha Boot Util")]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Asm,
    Execute(BootArgs),
    Flash(BootArgs),
}

static DIRS: OnceCell<Dirs> = OnceCell::new();

fn main() {
    let _ = DIRS.set(Dirs::new());
    use Commands::*;
    match Cli::parse().command {
        Asm => objdump(),
        Execute(args) => args.execute(),
        Flash(args) => args.flash(),
    }
}

struct Dirs {
    workspace: PathBuf,
    target: PathBuf,
}

impl Dirs {
    fn new() -> Self {
        let workspace = Path::new(std::env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let target = workspace.join("target/riscv64imac-unknown-none-elf/release");
        Self { workspace, target }
    }
}

fn make() {
    for package in ["bt0", "see"] {
        Cargo::build().package(package).release().invoke();
    }
}

fn objdump() {
    make();
    for package in ["bt0", "see"] {
        file(
            DIRS.wait().workspace.join(package).with_extension("asm"),
            BinUtil::objdump()
                .arg(DIRS.wait().target.join(package))
                .arg("-d")
                .output()
                .stdout,
        )
        .unwrap();
    }
}

#[derive(Args, Default)]
struct BootArgs {
    /// Target supervisor bin.
    #[clap(long)]
    kernel: Option<String>,
    /// Device tree file.
    #[clap(long)]
    dt: Option<String>,
}

impl BootArgs {
    fn make(&self) {
        let target = DIRS.wait().target.clone();
        for package in ["bt0", "see"] {
            Cargo::build().package(package).release().invoke();
            BinUtil::objcopy()
                .arg("--binary-architecture=riscv64")
                .arg(target.join(package))
                .args(["--strip-all", "-O", "binary"])
                .arg(target.join(package).with_extension("bin"))
                .invoke();
        }

        let bt0 = File::open(target.join("bt0").with_extension("bin")).unwrap();
        let see = File::open(target.join("see").with_extension("bin")).unwrap();
        let kernel = self.kernel.as_ref().map(|p| File::open(p).unwrap());
        let dtb = self.dt.as_ref().map(|dt| compile_dt(dt));

        let len_bt0 = bt0.metadata().unwrap().len();
        let len_see = see.metadata().unwrap().len();
        let len_kernel = kernel.map_or(0, |f| f.metadata().unwrap().len());
        let len_dtb = dtb.map_or(0, |f| f.metadata().unwrap().len());

        println!(
            "
| stage  | size          |
|--------|---------------|
| bt0    | {len_bt0:7} bytes |
| see    | {len_see:7} bytes |
| kernel | {len_kernel:7} bytes |
| dtb    | {len_dtb:7} bytes |
"
        );
    }

    fn execute(&self) {
        self.make();
        println!("execute");
    }

    fn flash(&self) {
        self.make();
        println!("flash");
    }
}

fn compile_dt(dt: impl AsRef<Path>) -> File {
    let dt = dt.as_ref();
    let dtb = if dt.extension() == Some(OsStr::new("dts")) {
        let dtb = DIRS.wait().target.join("nezha.dtb");
        Ext::new("dtc").arg("-o").arg(&dtb).arg(dt).invoke();
        dtb
    } else {
        dt.to_path_buf()
    };
    File::open(dtb).unwrap()
}
