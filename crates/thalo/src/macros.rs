#[macro_export]
macro_rules! export_aggregate {
    ($t: ident) => {
        mod __aggregate_export {
            use std::cell::RefCell;

            use $crate::__macro_helpers::*;

            pub type Agg = super::$t;

            wit_bindgen::generate!({
                inline: r#"
                    package thalo:aggregate;

                    world aggregate {
                        export aggregate: interface {
                            // record context {
                            //     id: u64,
                            //     stream-name: string,
                            //     position: u64,
                            //     metadata: string,
                            //     time: u64,
                            // }

                            record event {
                                // ctx: context,
                                event: string,
                                payload: string,
                            }

                            record command {
                                // ctx: context,
                                command: string,
                                payload: string,
                            }

                            variant error {
                                command(string),
                                ignore(option<string>),
                                deserialize-command(string),
                                deserialize-context(string),
                                deserialize-event(string),
                                serialize-event(string),
                            }

                            resource agg {
                                constructor(id: string);
                                apply: func(events: list<event>) -> result<_, error>;
                                handle: func(command: command) -> result<list<event>, error>;
                            }
                        }
                    }
                "#,
                exports: {
                    "aggregate/agg": AggWrapper
                }
            });

            use exports::aggregate as wit;

            pub struct AggWrapper(RefCell<Agg>);

            impl wit::GuestAgg for AggWrapper {
                fn new(id: String) -> Self {
                    init_aggregate(id)
                }

                fn apply(&self, events: Vec<wit::Event>) -> Result<(), wit::Error> {
                    apply_aggregate_events(self, events)
                }

                fn handle(&self, command: wit::Command) -> Result<Vec<wit::Event>, wit::Error> {
                    handle_aggregate_command(self, command)
                }
            }

            fn init_aggregate(id: String) -> AggWrapper {
                AggWrapper(RefCell::new(<Agg as $crate::Aggregate>::init(
                    <Agg as $crate::Aggregate>::ID::from(id),
                )))
            }

            fn apply_aggregate_events(
                AggWrapper(state): &AggWrapper,
                events: Vec<wit::Event>,
            ) -> Result<(), wit::Error> {
                let mut state = state.borrow_mut();
                for wit::Event {
                    // ctx,
                    event,
                    payload,
                } in events
                {
                    // let ctx = ctx.try_into()?;
                    let payload: serde_json::Value = serde_json::from_str(&payload)
                        .map_err(|err| wit::Error::DeserializeEvent(err.to_string()))?;
                    let event_value = serde_json::json!({
                        "event": event,
                        "payload": payload,
                    });
                    let evt = serde_json::from_value(event_value)
                        .map_err(|err| wit::Error::DeserializeEvent(err.to_string()))?;
                    <Agg as $crate::Aggregate>::apply(&mut state,
                        //ctx,
                         evt);
                }

                Ok(())
            }

            fn handle_aggregate_command(
                AggWrapper(state): &AggWrapper,
                wit::Command {
                    // ctx,
                    command,
                    payload,
                }: wit::Command,
            ) -> Result<Vec<wit::Event>, wit::Error> {
                let state = state.borrow();
                let payload: serde_json::Value = serde_json::from_str(&payload)
                    .map_err(|err| wit::Error::DeserializeCommand(err.to_string()))?;
                let event_value = serde_json::json!({
                    "command": command,
                    "payload": payload,
                });
                // let cmd_ctx = ctx.clone().try_into()?;
                let cmd = serde_json::from_value(event_value)
                    .map_err(|err| wit::Error::DeserializeCommand(err.to_string()))?;
                let events = <Agg as $crate::Aggregate>::handle(&state,
                    //cmd_ctx,
                    cmd)
                    .map_err(|err| wit::Error::Command(err.to_string()))?
                    .into_iter()
                    .map(|event| {
                        let event_value = serde_json::to_value(event)
                            .map_err(|err| wit::Error::SerializeEvent(err.to_string()))?;
                        let EventParts { event, payload } = serde_json::from_value(event_value)
                            .map_err(|err| wit::Error::SerializeEvent(err.to_string()))?;
                        let payload = serde_json::to_string(&payload).unwrap();
                        Ok(wit::Event {
                            // ctx: ctx.clone(),
                            event,
                            payload,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(events)
            }

            #[derive(serde::Deserialize)]
            struct EventParts {
                event: String,
                payload: serde_json::Value,
            }

            // impl TryFrom<wit::Context> for $crate::Context<'_> {
            //     type Error = wit::Error;

            //     fn try_from(ctx: wit::Context) -> Result<Self, Self::Error> {
            //         let stream_name = $crate::StreamName::new(ctx.stream_name).map_err(
            //             |err: $crate::EmptyStreamName| wit::Error::DeserializeContext(err.to_string()),
            //         )?;
            //         let metadata = serde_json::from_str(&ctx.metadata)
            //             .map_err(|err| wit::Error::DeserializeContext(err.to_string()))?;
            //         let time = std::time::UNIX_EPOCH + std::time::Duration::from_millis(ctx.time);
            //         Ok($crate::Context {
            //             id: ctx.id,
            //             stream_name,
            //             position: ctx.position,
            //             metadata,
            //             time,
            //         })
            //     }
            // }
        }
    };
}

macro_rules! impl_eq {
    ($lhs:ty, $rhs: ty) => {
        #[allow(unused_lifetimes)]
        impl<'a, 'b> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                PartialEq::eq(&(&*self)[..], &other[..])
            }
            #[inline]
            fn ne(&self, other: &$rhs) -> bool {
                PartialEq::ne(&self[..], &other[..])
            }
        }

        #[allow(unused_lifetimes)]
        impl<'a, 'b> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
            #[inline]
            fn ne(&self, other: &$lhs) -> bool {
                PartialEq::ne(&self[..], &other[..])
            }
        }
    };
}

macro_rules! impl_as_ref_str {
    ($tp:expr, $t:ty, $tt:ty) => {
        impl<'a> $t {
            pub fn into_owned(self) -> $tt {
                $tp(self.into_string().into())
            }

            pub fn into_string(self) -> String {
                self.0.into_owned()
            }

            pub fn as_borrowed(&'a self) -> $t {
                Self(Cow::Borrowed(&self.0))
            }
        }

        impl<'a> std::fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl<'a> AsRef<str> for $t {
            fn as_ref(&self) -> &str {
                &self.0.as_ref()
            }
        }

        impl<'a> AsRef<[u8]> for $t {
            fn as_ref(&self) -> &[u8] {
                self.0.as_bytes()
            }
        }

        impl<'a> AsRef<std::borrow::Cow<'a, str>> for $t {
            fn as_ref(&self) -> &std::borrow::Cow<'a, str> {
                &self.0
            }
        }

        impl<'a> From<$t> for String {
            fn from(v: $t) -> String {
                v.0.into()
            }
        }

        impl<'a> std::ops::Deref for $t {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.0.deref()
            }
        }
    };
}
