mod clocks;
mod filesystem;
mod logging;
mod poll;
mod random;
mod table;
use rand::rngs::StdRng;
use rand::SeedableRng;
pub use table::Table;

wasmtime::component::bindgen!({
    path: "../../wit/wasi.wit",
    tracing: true,
});

pub fn add_to_linker<T>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl (Fn(&mut T) -> &mut WasiCtx) + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    wasi_clocks::add_to_linker(l, f)?;
    wasi_default_clocks::add_to_linker(l, f)?;
    wasi_filesystem::add_to_linker(l, f)?;
    wasi_logging::add_to_linker(l, f)?;
    wasi_poll::add_to_linker(l, f)?;
    wasi_random::add_to_linker(l, f)?;
    Ok(())
}

pub struct WasiCtx {
    table: Table,
    default_monotonic: wasi_clocks::MonotonicClock,
    default_wall: wasi_clocks::WallClock,
    rng: StdRng,
}

impl Default for WasiCtx {
    fn default() -> WasiCtx {
        let mut table = Table::default();
        let default_monotonic = table
            .push(Box::new(clocks::MonotonicClock::default()))
            .unwrap();
        let default_wall = table.push(Box::new(clocks::WallClock::default())).unwrap();
        WasiCtx {
            table,
            default_monotonic,
            default_wall,
            rng: StdRng::from_entropy(),
        }
    }
}
