#![allow(missing_docs)]

use std::{collections::HashMap, fmt};

use linked_hash_map::LinkedHashMap;
use serde::{
    de::{self, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize,
};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Aggregate {
    pub name: String,
    pub commands: HashMap<String, Command>,
    pub events: HashMap<String, Event>,
    pub errors: HashMap<String, Error>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub params: Fields,
    pub event: Option<(String, Event)>,
    pub infallible: bool,
    pub optional: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Event {
    #[serde(default)]
    pub fields: Fields,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Fields(pub LinkedHashMap<String, Field>);

impl From<LinkedHashMap<String, Field>> for Fields {
    fn from(fields: LinkedHashMap<String, Field>) -> Self {
        Fields(fields)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Simple(FieldType),
    Raw(String),
    Optional(FieldType),         // Option<T>
    Repeated(FieldType),         // Vec<T>
    OptionalRepeated(FieldType), // Option<Vec<T>>
    RepeatedOptional(FieldType), // Vec<Option<T>>
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Integer, // Can also be aliased to number
    Float,
    Bool,
    Timestamp,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Error {
    pub message: String,
}

impl Field {
    fn from_str(s: &str) -> Result<Self, ()> {
        if let Some(ty) = s.strip_prefix("r#") {
            return Ok(Field::Raw(ty.to_string()));
        }

        let (field_type, tail) = if let Some(tail) = s.strip_prefix("string") {
            (FieldType::String, tail)
        } else if let Some(tail) = s.strip_prefix("integer") {
            (FieldType::Integer, tail)
        } else if let Some(tail) = s.strip_prefix("number") {
            (FieldType::Integer, tail)
        } else if let Some(tail) = s.strip_prefix("float") {
            (FieldType::Float, tail)
        } else if let Some(tail) = s.strip_prefix("bool") {
            (FieldType::Bool, tail)
        } else if let Some(tail) = s.strip_prefix("timestamp") {
            (FieldType::Timestamp, tail)
        } else {
            return Err(());
        };

        let field = match tail {
            "?" => Field::Optional(field_type),
            "[]" => Field::Repeated(field_type),
            "[]?" => Field::OptionalRepeated(field_type),
            "?[]" => Field::RepeatedOptional(field_type),
            "" => Field::Simple(field_type),
            _ => return Err(()),
        };

        Ok(field)
    }

    pub fn to_rust_type(&self) -> String {
        use Field::*;

        match self {
            Simple(field_type) => field_type.to_rust_type().to_string(),
            Raw(ty) => ty.to_string(),
            Optional(ty) => format!("std::option::Option<{}>", ty.to_rust_type()),
            Repeated(ty) => format!("std::vec::Vec<{}>", ty.to_rust_type()),
            OptionalRepeated(ty) => {
                format!("std::option::Option<std::vec::Vec<{}>>", ty.to_rust_type())
            }
            RepeatedOptional(ty) => {
                format!("std::vec::Vec<std::option::Option<{}>>", ty.to_rust_type())
            }
        }
    }
}

impl FieldType {
    fn to_rust_type(&self) -> &'static str {
        use FieldType::*;

        match self {
            String => "String",
            Integer => "i64",
            Float => "f64",
            Bool => "bool",
            Timestamp => "chrono::DateTime<chrono::Utc>",
        }
    }
}

impl<'de> Deserialize<'de> for Fields {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldKeyValue(String, Field);

        impl<'de> Deserialize<'de> for FieldKeyValue {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldKeyValueVisitor;

                impl<'de> Visitor<'de> for FieldKeyValueVisitor {
                    type Value = FieldKeyValue;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("field name and type")
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: de::MapAccess<'de>,
                    {
                        let (key, value) = map
                            .next_entry::<String, String>()?
                            .ok_or_else(|| de::Error::missing_field("field name and type"))?;

                        let field = Field::from_str(&value).map_err(|_| de::Error::invalid_value(Unexpected::Str(&value), &"string | integer/number | float | bool | timestamp with an optional `[]` or `?` suffix"))?;

                        Ok(FieldKeyValue(key, field))
                    }
                }

                deserializer.deserialize_map(FieldKeyValueVisitor)
            }
        }

        struct FieldsVisitor;

        impl<'de> Visitor<'de> for FieldsVisitor {
            type Value = Fields;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("fields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut fields = Fields(LinkedHashMap::with_capacity(seq.size_hint().unwrap_or(0)));
                while let Some(element) = seq.next_element::<FieldKeyValue>()? {
                    fields.0.insert(element.0, element.1);
                }
                Ok(fields)
            }
        }

        deserializer.deserialize_seq(FieldsVisitor)
    }
}

impl<'de> Deserialize<'de> for Aggregate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct NamedCommand {
            params: Fields,
            event: Option<String>,
            infallible: bool,
            optional: bool,
        }

        struct AggregateVisitor;

        impl<'de> Visitor<'de> for AggregateVisitor {
            type Value = Aggregate;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Aggregate")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut name = None;
                let mut named_commands: Option<HashMap<String, NamedCommand>> = None;
                let mut events = None;
                let mut errors = None;

                for _ in 0..4 {
                    match map.next_key::<String>()?.as_deref() {
                        Some("name") => {
                            name = Some(map.next_value()?);
                        }
                        Some("commands") => {
                            named_commands = Some(map.next_value()?);
                        }
                        Some("events") => {
                            events = Some(map.next_value()?);
                        }
                        Some("errors") => {
                            errors = Some(map.next_value()?);
                        }
                        Some(name) => {
                            return Err(de::Error::unknown_field(
                                name,
                                &["name", "commands", "events", "errors"],
                            ))
                        }
                        _ => {}
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let named_commands =
                    named_commands.ok_or_else(|| de::Error::missing_field("commands"))?;
                let events: HashMap<String, Event> =
                    events.ok_or_else(|| de::Error::missing_field("events"))?;
                let errors = errors.unwrap_or_default();

                let commands = named_commands
                    .into_iter()
                    .map(|(name, command)| {
                        let event = command
                            .event
                            .map(|event| {
                                let ev = events
                                    .get(&event)
                                    .ok_or_else(|| {
                                        de::Error::invalid_value(
                                            Unexpected::Str(&event),
                                            &"a reference to defined event",
                                        )
                                    })?
                                    .clone();

                                Ok((event, ev))
                            })
                            .transpose()?;

                        if command.optional && event.is_none() {
                            return Err(de::Error::invalid_value(
                                Unexpected::Bool(command.optional),
                                &"`false` when `event` is not specified",
                            ));
                        }

                        Ok((
                            name,
                            Command {
                                params: command.params,
                                event,
                                infallible: command.infallible,
                                optional: command.optional,
                            },
                        ))
                    })
                    .collect::<Result<_, _>>()?;

                Ok(Aggregate {
                    name,
                    commands,
                    events,
                    errors,
                })
            }
        }

        deserializer.deserialize_struct(
            "Aggregate",
            &["name", "commands", "events", "errors"],
            AggregateVisitor,
        )
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use linked_hash_map::LinkedHashMap;

    use crate::schema::{Aggregate, Command, Error, Event, Field, FieldType};

    #[test]
    fn it_deserializes_correctly_debug() -> Result<(), serde_yaml::Error> {
        let config = r#"
name: Aggregate

events:
  Created:
    fields:
      - project_id: string
      - description: string?

commands:
  create:
    event: Created
    params:
      - project_id: string
      - description: string?

errors:
  AlreadyCreated:
    message: "aggregate already created"
        "#;

        let aggregate = serde_yaml::from_str::<Aggregate>(config)?;

        println!("{:#?}", aggregate);
        println!("{:#?}", aggregate.commands.get("create").unwrap().params);

        Ok(())
    }

    #[test]
    fn it_deserializes_correctly() -> Result<(), serde_yaml::Error> {
        let config = r#"
        # The name of the aggregate
        name: BankAccount

        # The events resulted resulting from commands
        events:
          AccountOpened:
            fields:
              initial_balance: float

          DepositedFunds:
            fields:
              amount: float

          WithdrewFunds:
            fields:
              amount: float

        # The commands availble for the aggregate
        commands:
          open_account:
            event: AccountOpened # resulting event when command is successful
            params:
              initial_balance: float

          deposit_funds:
            event: DepositedFunds
            params:
              amount: float

          withdraw_funds:
            event: WithdrewFunds
            params:
                amount: float

        # Errors that can occur when executing a command
        errors:
          NegativeAmount:
            message: "amount cannot be negative"

          InsufficientFunds:
            message: "insufficient funds for transaction"
        "#;

        let aggregate = serde_yaml::from_str::<Aggregate>(config)?;

        let name = "BankAccount".to_string();

        let mut events = HashMap::new();

        let account_opened = {
            let mut fields = LinkedHashMap::with_capacity(1);
            fields.insert(
                "initial_balance".to_string(),
                Field::Simple(FieldType::Float),
            );
            Event {
                fields: fields.into(),
            }
        };
        let deposited_funds = {
            let mut fields = LinkedHashMap::with_capacity(1);
            fields.insert("amount".to_string(), Field::Simple(FieldType::Float));
            Event {
                fields: fields.into(),
            }
        };
        let withdrew_funds = {
            let mut fields = LinkedHashMap::with_capacity(1);
            fields.insert("amount".to_string(), Field::Simple(FieldType::Float));
            Event {
                fields: fields.into(),
            }
        };
        events.insert("AccountOpened".to_string(), account_opened.clone());
        events.insert("DepositedFunds".to_string(), deposited_funds.clone());
        events.insert("WithdrewFunds".to_string(), withdrew_funds.clone());

        let mut commands = HashMap::new();
        let open_account = {
            let mut params = LinkedHashMap::with_capacity(1);
            params.insert(
                "initial_balance".to_string(),
                Field::Simple(FieldType::Float),
            );
            Command {
                event: Some(("AccountOpened".to_string(), account_opened)),
                infallible: false,
                optional: false,
                params: params.into(),
            }
        };
        commands.insert("open_account".to_string(), open_account);
        let deposit_funds = {
            let mut params = LinkedHashMap::with_capacity(1);
            params.insert("amount".to_string(), Field::Simple(FieldType::Float));
            Command {
                event: Some(("DepositedFunds".to_string(), deposited_funds)),
                infallible: false,
                optional: false,
                params: params.into(),
            }
        };
        commands.insert("deposit_funds".to_string(), deposit_funds);
        let withdraw_funds = {
            let mut params = LinkedHashMap::with_capacity(1);
            params.insert("amount".to_string(), Field::Simple(FieldType::Float));
            Command {
                event: Some(("WithdrewFunds".to_string(), withdrew_funds)),
                infallible: false,
                optional: false,
                params: params.into(),
            }
        };
        commands.insert("withdraw_funds".to_string(), withdraw_funds);

        let mut errors = HashMap::new();
        errors.insert(
            "NegativeAmount".to_string(),
            Error {
                message: "amount cannot be negative".to_string(),
            },
        );
        errors.insert(
            "InsufficientFunds".to_string(),
            Error {
                message: "insufficient funds for transaction".to_string(),
            },
        );

        assert_eq!(
            aggregate,
            Aggregate {
                name,
                events,
                commands,
                errors,
            }
        );

        Ok(())
    }
}
