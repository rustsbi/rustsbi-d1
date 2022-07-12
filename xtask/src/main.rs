#[macro_use]
extern crate clap;

use clap::Parser;
use command_ext::{BinUtil, Cargo, CommandExt};
use once_cell::sync::OnceCell;
use std::{
    fs::{self, write as file},
    path::{Path, PathBuf},
    str::FromStr,
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
    Asm(AsmArgs),
    Boot,
    Debug(DebugArg),
}

static DIRS: OnceCell<Dirs> = OnceCell::new();

fn main() {
    let _ = DIRS.set(Dirs::new());
    use Commands::*;
    match Cli::parse().command {
        Asm(args) => args.asm(),
        Boot => todo!(),
        Debug(arg) => println!("{:?}", arg.stage),
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Sram,
    Dram,
}

impl FromStr for Stage {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sram" => Ok(Self::Sram),
            "dram" => Ok(Self::Dram),
            _ => Err("unknown stage"),
        }
    }
}

#[derive(Args)]
struct DebugArg {
    #[clap(long)]
    stage: Option<Stage>,
}

#[derive(Args)]
struct AsmArgs {
    #[clap(long)]
    stage: Option<Stage>,
    #[clap(short, long)]
    output: Option<PathBuf>,
}

impl AsmArgs {
    fn asm(&self) {
        let output = self
            .output
            .as_ref()
            .unwrap_or(&DIRS.wait().workspace)
            .as_path();
        if let Some(stage) = self.stage {
            let package = match stage {
                Stage::Sram => "bt0",
                Stage::Dram => "see",
            };
            let path = if output.is_dir() {
                output.join(package).with_extension("asm")
            } else {
                output.to_path_buf()
            };
            Cargo::build().package(package).release().invoke();
            let contents = BinUtil::objdump()
                .arg(DIRS.wait().target.join(package))
                .arg("-d")
                .output()
                .stdout;
            file(path, contents).unwrap();
        } else {
            if output.is_dir() {
            } else if !output.exists() {
                fs::create_dir_all(output).unwrap();
            } else {
                panic!("output path must be a directory");
            }
            for package in ["bt0", "see"] {
                Cargo::build().package(package).release().invoke();
                file(
                    output.join(package).with_extension("asm"),
                    BinUtil::objdump()
                        .arg(DIRS.wait().target.join(package))
                        .arg("-d")
                        .output()
                        .stdout,
                )
                .unwrap();
            }
        }
    }
}

// fn make() {
//     for package in ["bt0", "see"] {
//         Cargo::build().package(package).release().invoke();
//     }
// }

// fn objdump() {
//     make();
//     for package in ["bt0", "see"] {
//         file(
//             DIRS.wait().workspace.join(package).with_extension("asm"),
//             BinUtil::objdump()
//                 .arg(DIRS.wait().target.join(package))
//                 .arg("-d")
//                 .output()
//                 .stdout,
//         )
//         .unwrap();
//     }
// }

// #[derive(Args, Default)]
// struct BootArgs {
//     /// Target supervisor bin.
//     #[clap(long)]
//     kernel: Option<String>,
//     /// Device tree file.
//     #[clap(long)]
//     dt: Option<String>,
// }

// impl BootArgs {
//     fn make(&self) {
//         let target = DIRS.wait().target.clone();
//         for package in ["bt0", "see"] {
//             Cargo::build().package(package).release().invoke();
//             BinUtil::objcopy()
//                 .arg("--binary-architecture=riscv64")
//                 .arg(target.join(package))
//                 .args(["--strip-all", "-O", "binary"])
//                 .arg(target.join(package).with_extension("bin"))
//                 .invoke();
//         }

//         let bt0 = File::open(target.join("bt0").with_extension("bin")).unwrap();
//         let see = File::open(target.join("see").with_extension("bin")).unwrap();
//         let kernel = self.kernel.as_ref().map(|p| File::open(p).unwrap());
//         let dtb = self.dt.as_ref().map(|dt| compile_dt(dt));

//         let len_bt0 = bt0.metadata().unwrap().len();
//         let len_see = see.metadata().unwrap().len();
//         let len_kernel = kernel.map_or(0, |f| f.metadata().unwrap().len());
//         let len_dtb = dtb.map_or(0, |f| f.metadata().unwrap().len());

//         println!(
//             "
// | stage  | size          |
// |--------|---------------|
// | bt0    | {len_bt0:7} bytes |
// | see    | {len_see:7} bytes |
// | kernel | {len_kernel:7} bytes |
// | dtb    | {len_dtb:7} bytes |
// "
//         );
//     }

//     fn execute(&self) {
//         self.make();
//         println!("execute");
//     }

//     fn flash(&self) {
//         self.make();
//         println!("flash");
//     }
// }

// fn compile_dt(dt: impl AsRef<Path>) -> File {
//     let dt = dt.as_ref();
//     let dtb = if dt.extension() == Some(OsStr::new("dts")) {
//         let dtb = DIRS.wait().target.join("nezha.dtb");
//         Ext::new("dtc").arg("-o").arg(&dtb).arg(dt).invoke();
//         dtb
//     } else {
//         dt.to_path_buf()
//     };
//     File::open(dtb).unwrap()
// }
