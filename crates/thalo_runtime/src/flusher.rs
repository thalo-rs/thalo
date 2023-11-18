use std::{pin::Pin, time::Duration};

use async_trait::async_trait;
use futures::Future;
use ractor::{concurrency::JoinHandle, Actor, ActorCell, ActorProcessingErr, ActorRef, SpawnErr};

pub type FlusherFn = Box<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), ActorProcessingErr>> + Send>> + Send + Sync,
>;

pub type FlusherRef = ActorRef<FlusherMsg>;

pub struct Flusher;

impl Flusher {
    pub async fn spawn_linked<F, Fu>(
        name: impl Into<Option<String>>,
        supervisor: ActorCell,
        interval: Duration,
        f: F,
    ) -> Result<(FlusherRef, JoinHandle<()>), SpawnErr>
    where
        F: Fn() -> Fu + Send + Sync + 'static,
        Fu: Future<Output = Result<(), ActorProcessingErr>> + Send + 'static,
    {
        Actor::spawn_linked(
            name.into(),
            Flusher,
            (
                interval,
                Box::new(move || {
                    let future = f();
                    Box::pin(async move { future.await })
                        as Pin<Box<dyn Future<Output = Result<(), ActorProcessingErr>> + Send>>
                }),
            ),
            supervisor,
        )
        .await
    }
}

pub struct FlusherState {
    f: FlusherFn,
    interval: Duration,
    is_dirty: bool,
}

pub enum FlusherMsg {
    Flush,
    MarkDirty,
}

#[async_trait]
impl Actor for Flusher {
    type State = FlusherState;
    type Msg = FlusherMsg;
    type Arguments = (Duration, FlusherFn);

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        (interval, f): Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        myself.send_after(interval, || FlusherMsg::Flush);

        Ok(FlusherState {
            f,
            interval,
            is_dirty: false,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        FlusherState {
            f,
            interval,
            is_dirty,
        }: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            FlusherMsg::Flush => {
                if *is_dirty {
                    f().await?;
                    *is_dirty = false;
                }
                myself.send_after(*interval, || FlusherMsg::Flush);
            }
            FlusherMsg::MarkDirty => {
                *is_dirty = true;
            }
        }

        Ok(())
    }
}
