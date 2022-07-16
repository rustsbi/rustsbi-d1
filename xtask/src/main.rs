mod asm;
// mod xfel;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

use asm::AsmArg;
use clap::Parser;
use command_ext::{BinUtil, Cargo, CommandExt, Ext};
use once_cell::sync::Lazy;
use std::{
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    io::{Error as IoError, ErrorKind as IoErrorKind},
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[clap(name = "NeZha Boot Util")]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
    #[clap(flatten)]
    components: Components,
}

#[derive(Subcommand)]
enum Commands {
    Make,
    Asm(AsmArg),
    Debug,
    Flash,
}

static DIRS: Lazy<Dirs> = Lazy::new(Dirs::new);

fn main() -> Result<(), Box<dyn Error>> {
    use simplelog::*;
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    use Commands::*;
    let cli = Cli::parse();
    match cli.command {
        Make => make(cli.components).is_ok(),
        Asm(arg) => arg.asm(cli.components),
        Debug => todo!(),
        Flash => todo!(),
    };
    Ok(())
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

enum Package {
    Spl,
    See,
}

impl Package {
    #[inline]
    const fn name(&self) -> &'static str {
        match self {
            Self::Spl => "spl",
            Self::See => "see",
        }
    }

    #[inline]
    fn build(&self) {
        info!("building `{}`", self.name());
        Cargo::build().package(self.name()).release().invoke();
    }

    #[inline]
    fn target(&self) -> PathBuf {
        DIRS.target.join(self.name())
    }

    fn objdump(&self) -> Vec<u8> {
        self.build();
        BinUtil::objdump()
            .arg(self.target())
            .arg("-d")
            .output()
            .stdout
    }

    fn objcopy(&self) -> PathBuf {
        self.build();
        let target = self.target();
        let bin = target.with_extension("bin");
        info!("strip `{}` to {}", self.name(), bin.display());
        BinUtil::objcopy()
            .arg("--binary-architecture=riscv64")
            .arg(target)
            .args(["--strip-all", "-O", "binary"])
            .arg(&bin)
            .invoke();
        bin
    }
}

#[derive(Args)]
struct Components {
    #[clap(long, global = true)]
    spl: bool,
    #[clap(long, global = true)]
    see: bool,
    #[clap(long, global = true)]
    kernel: Option<PathBuf>,
    #[clap(long, global = true)]
    dt: Option<PathBuf>,
}

#[derive(Default)]
struct Target {
    spl: Option<File>,
    see: Option<File>,
    kernel: Option<File>,
    dtb: Option<File>,
}

fn make(components: Components) -> Result<Target, Box<dyn Error>> {
    let mut ans = Target::default();
    if components.spl {
        ans.spl.replace(fs::File::open(Package::Spl.objcopy())?);
    }
    if components.see {
        ans.see.replace(fs::File::open(Package::See.objcopy())?);
    }
    if let Some(kernel) = components.kernel {
        ans.kernel.replace(fs::File::open(kernel)?);
    }
    if let Some(dt) = components.dt {
        if !dt.is_file() {
            return Err(IoError::new(
                IoErrorKind::NotFound,
                format!("dt file \"{}\" not exist", dt.display()),
            )
            .into());
        }
        if dt.extension() == Some(OsStr::new("dts")) {
            let dtb = DIRS
                .target
                .join(dt.file_stem().unwrap_or(OsStr::new("nezha")))
                .with_extension("dtb");
            Ext::new("dtc").arg("-o").arg(&dtb).arg(&dt).invoke();
            ans.dtb.replace(fs::File::open(dtb)?);
        } else {
            ans.dtb.replace(fs::File::open(dt)?);
        }
    }
    Ok(ans)
}
