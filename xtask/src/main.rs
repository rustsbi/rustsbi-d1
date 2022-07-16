mod components;
mod xfel;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

use clap::Parser;
use command_ext::{BinUtil, Cargo, CommandExt};
use components::Components;
use once_cell::sync::Lazy;
use std::{
    error::Error,
    fmt::{Debug, Display},
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
        Make => cli.components.make().map(|_| ()),
        Asm(arg) => cli.components.asm(arg),
        Debug => cli.components.debug(),
        Flash => todo!(),
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
        info!("build `{}`", self.name());
        Cargo::build().package(self.name()).release().invoke();
    }

    #[inline]
    fn target(&self) -> PathBuf {
        DIRS.target.join(self.name())
    }

    fn objdump(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        self.build();
        let path = path.as_ref();
        info!("dump `{}` to {}", self.name(), path.display());
        std::fs::write(
            path,
            BinUtil::objdump()
                .arg(self.target())
                .arg("-d")
                .output()
                .stdout,
        )?;
        Ok(())
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

#[derive(Default)]
struct Target {
    spl: Option<PathBuf>,
    see: Option<PathBuf>,
    kernel: Option<PathBuf>,
    dtb: Option<PathBuf>,
}

#[derive(Args)]
struct AsmArg {
    #[clap(short, long)]
    output: Option<PathBuf>,
}

#[derive(Debug)]
enum XError {
    InvalidProcedure(String),
}

impl Error for XError {}

impl Display for XError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
