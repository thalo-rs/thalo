use std::{
    borrow::Cow,
    time::{Duration, UNIX_EPOCH},
};

use anyhow::Context;
use thalo::StreamName;
use thiserror::Error;

tonic::include_proto!("thalo");

impl TryFrom<thalo_message_store::GenericMessage<'static>> for Message {
    type Error = NonF64NumberError;

    fn try_from(msg: thalo_message_store::GenericMessage<'static>) -> Result<Self, Self::Error> {
        Ok(Message {
            id: msg.id,
            stream_name: msg.stream_name.into_string(),
            msg_type: msg.msg_type.into_owned(),
            position: msg.position,
            global_position: msg.global_position,
            data: Some(json_value_to_prost_value(msg.data.into_owned())?),
            metadata: Some(Metadata::try_from(msg.metadata)?),
            time: msg
                .time
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }
}

impl TryFrom<thalo::Metadata<'static>> for Metadata {
    type Error = NonF64NumberError;

    fn try_from(metadata: thalo::Metadata<'static>) -> Result<Self, Self::Error> {
        Ok(Metadata {
            stream_name: metadata.stream_name.into_string(),
            position: metadata.position,
            reply_stream_name: metadata.reply_stream_name.map(StreamName::into_string),
            properties: metadata
                .properties
                .into_iter()
                .map(|(k, v)| Ok((k.into_owned(), json_value_to_prost_value(v.into_owned())?)))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl TryFrom<Message> for thalo_message_store::GenericMessage<'static> {
    type Error = anyhow::Error;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        Ok(thalo_message_store::GenericMessage {
            id: msg.id,
            stream_name: StreamName::new(msg.stream_name)?,
            msg_type: Cow::Owned(msg.msg_type),
            position: msg.position,
            global_position: msg.global_position,
            data: Cow::Owned(
                msg.data
                    .map(|data| prost_value_to_json_value(data))
                    .transpose()?
                    .unwrap_or(serde_json::Value::Null),
            ),
            metadata: msg
                .metadata
                .map(thalo::Metadata::try_from)
                .transpose()?
                .context("missing metadata")?,
            time: UNIX_EPOCH + Duration::from_millis(msg.time),
        })
    }
}

impl TryFrom<Metadata> for thalo::Metadata<'static> {
    type Error = anyhow::Error;

    fn try_from(metadata: Metadata) -> Result<Self, Self::Error> {
        Ok(thalo::Metadata {
            stream_name: StreamName::new(metadata.stream_name)?,
            position: metadata.position,
            reply_stream_name: metadata
                .reply_stream_name
                .map(StreamName::new)
                .transpose()?,
            properties: metadata
                .properties
                .into_iter()
                .map(|(k, v)| {
                    Ok::<_, anyhow::Error>((
                        Cow::Owned(k),
                        Cow::Owned(prost_value_to_json_value(v)?),
                    ))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Clone, Copy, Debug, Error)]
#[error("non-f64-representable number")]
pub struct NonF64NumberError;

pub(super) fn json_map_to_prost_struct(
    json: serde_json::Map<String, serde_json::Value>,
) -> Result<prost_types::Struct, NonF64NumberError> {
    Ok(prost_types::Struct {
        fields: json
            .into_iter()
            .map(|(k, v)| Ok((k, json_value_to_prost_value(v)?)))
            .collect::<Result<_, NonF64NumberError>>()?,
    })
}

pub(super) fn json_value_to_prost_value(
    json: serde_json::Value,
) -> Result<prost_types::Value, NonF64NumberError> {
    use prost_types::value::Kind::*;
    use serde_json::Value::*;

    Ok(prost_types::Value {
        kind: Some(match json {
            Null => NullValue(0 /* wat? */),
            Bool(v) => BoolValue(v),
            Number(n) => NumberValue(n.as_f64().ok_or(NonF64NumberError)?),
            String(s) => StringValue(s),
            Array(v) => ListValue(prost_types::ListValue {
                values: v
                    .into_iter()
                    .map(json_value_to_prost_value)
                    .collect::<Result<_, _>>()?,
            }),
            Object(v) => StructValue(json_map_to_prost_struct(v)?),
        }),
    })
}

#[derive(Clone, Copy, Debug, Error)]
#[error("infinite or NaN number")]
pub(super) struct InfiniteOrNaNNumberError;

pub(super) fn prost_struct_to_json_map(
    s: prost_types::Struct,
) -> Result<serde_json::Map<String, serde_json::Value>, InfiniteOrNaNNumberError> {
    s.fields
        .into_iter()
        .map(|(k, v)| Ok((k, prost_value_to_json_value(v)?)))
        .collect::<Result<_, InfiniteOrNaNNumberError>>()
}

pub(super) fn prost_value_to_json_value(
    value: prost_types::Value,
) -> Result<serde_json::Value, InfiniteOrNaNNumberError> {
    use prost_types::value::Kind::*;
    use serde_json::Value::*;

    Ok(match value.kind {
        Some(value) => match value {
            NullValue(_) => Null,
            NumberValue(n) => {
                Number(serde_json::Number::from_f64(n).ok_or(InfiniteOrNaNNumberError)?)
            }
            StringValue(s) => String(s),
            BoolValue(b) => Bool(b),
            StructValue(s) => Object(prost_struct_to_json_map(s)?),
            ListValue(l) => Array(
                l.values
                    .into_iter()
                    .map(prost_value_to_json_value)
                    .collect::<Result<_, _>>()?,
            ),
        },
        None => Null,
    })
}
