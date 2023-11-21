use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
};

use anyhow::{bail, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use clap::Args;
use tracing::debug;
use wit_component::ComponentEncoder;

use crate::terminal::{Color, Terminal, Verbosity};

/// Builds an aggreagate to a wasm component using cargo
#[derive(Args, Clone, Debug)]
pub struct Build {
    /// Name of aggregate
    name: String,
    /// Output path/file
    #[clap(short, long, default_value = "./")]
    output: PathBuf,
}

impl Build {
    pub async fn build(self) -> Result<()> {
        let terminal = Terminal::new(Verbosity::Normal, Color::Auto);

        let metadata = load_metadata()?;

        let cargo = env::var("CARGO")
            .map(PathBuf::from)
            .ok()
            .unwrap_or_else(|| PathBuf::from("cargo"));

        let mut cmd = Command::new(&cargo);
        cmd.arg("rustc")
            .arg("-p")
            .arg(&self.name)
            .arg("--lib")
            .arg("--target")
            .arg("wasm32-wasi")
            .arg("--release")
            .arg("--crate-type=cdylib");

        install_wasm32_wasi(&terminal)?;

        match cmd.status() {
            Ok(status) => {
                if !status.success() {
                    process::exit(status.code().unwrap_or(1));
                }
            }
            Err(e) => {
                bail!("failed to spawn `{cargo}`: {e}", cargo = cargo.display());
            }
        }

        debug!("searching for WebAssembly module to componentize");
        let out_dir = metadata
            .target_directory
            .join("wasm32-wasi")
            .join("release");

        let path = out_dir.join(&self.name).with_extension("wasm");
        if !path.exists() {
            bail!("Wasm file not found after build at {path}");
        }

        let output = match self.output.extension() {
            Some(_) => self.output,
            None => self.output.join(format!("{}.wasm", self.name)),
        };
        create_component(&terminal, path.as_std_path(), &output)
    }
}

fn load_metadata() -> Result<Metadata> {
    let mut command = MetadataCommand::new();
    command.no_deps();

    debug!("loading metadata from current directory");

    let metadata = command.exec().context("failed to load cargo metadata")?;

    Ok(metadata)
}

fn create_component(terminal: &Terminal, path: &Path, output: &Path) -> Result<()> {
    debug!(
        "componentizing WebAssembly module `{path}` as a {kind} component",
        path = path.display(),
        kind = "reactor",
    );

    let module = fs::read(path).with_context(|| {
        format!(
            "failed to read output module `{path}`",
            path = path.display()
        )
    })?;

    terminal.status(
        "Creating",
        format!("component {path}", path = path.display()),
    )?;

    let encoder = ComponentEncoder::default()
        .module(&module)?
        .adapter(
            "wasi_snapshot_preview1",
            include_bytes!("../../wasi_snapshot_preview1.wasm"),
        )
        .with_context(|| {
            format!(
                "failed to load adapter module `{path}`",
                path = Path::new("<built-in>").display()
            )
        })?
        .validate(true);

    let mut producers = wasm_metadata::Producers::empty();
    producers.add(
        "processed-by",
        env!("CARGO_PKG_NAME"),
        option_env!("CARGO_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION")),
    );

    let component = producers.add_to_wasm(&encoder.encode()?).with_context(|| {
        format!(
            "failed to add metadata to output component `{path}`",
            path = path.display()
        )
    })?;

    fs::write(output, component).with_context(|| {
        format!(
            "failed to write output component `{path}`",
            path = path.display()
        )
    })
}

fn install_wasm32_wasi(terminal: &Terminal) -> Result<()> {
    let sysroot = get_sysroot()?;
    if sysroot.join("lib/rustlib/wasm32-wasi").exists() {
        return Ok(());
    }

    if env::var_os("RUSTUP_TOOLCHAIN").is_none() {
        bail!(
            "failed to find the `wasm32-wasi` target \
             and `rustup` is not available. If you're using rustup \
             make sure that it's correctly installed; if not, make sure to \
             install the `wasm32-wasi` target before using this command"
        );
    }

    terminal.status("Installing", "wasm32-wasi target")?;

    let output = Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg("wasm32-wasi")
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .output()?;

    if !output.status.success() {
        bail!("failed to install the `wasm32-wasi` target");
    }

    Ok(())
}

fn get_sysroot() -> Result<PathBuf> {
    let output = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()?;

    if !output.status.success() {
        bail!(
            "failed to execute `rustc --print sysroot`, \
                 command exited with error: {output}",
            output = String::from_utf8_lossy(&output.stderr)
        );
    }

    let sysroot = PathBuf::from(String::from_utf8(output.stdout)?.trim());

    Ok(sysroot)
}
