#![allow(unused_variables)]

use anyhow::Result;

use super::wasi_poll::{WasiFuture, WasiPoll};
use super::WasiCtx;

impl WasiPoll for WasiCtx {
    fn drop_future(&mut self, future: WasiFuture) -> Result<()> {
        todo!()
    }

    fn poll_oneoff(&mut self, futures: Vec<WasiFuture>) -> Result<Vec<u8>> {
        todo!()
    }
}
