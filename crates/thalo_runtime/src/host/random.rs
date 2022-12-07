#![allow(unused_variables)]
use rand::distributions::Standard;
use rand::Rng;

use super::{wasi_random, WasiCtx};

impl wasi_random::WasiRandom for WasiCtx {
    fn getrandom(&mut self, len: u32) -> anyhow::Result<Vec<u8>> {
        Ok((&mut self.rng)
            .sample_iter(Standard)
            .take(len as usize)
            .collect())
    }
}
