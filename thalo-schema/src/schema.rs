#![allow(missing_docs)]

use std::{collections::BTreeMap, fmt};

use serde::{
    de::{self, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct Aggregate {
    pub name: String,
    pub commands: BTreeMap<String, Command>,
    pub events: BTreeMap<String, Event>,
    pub errors: BTreeMap<String, Error>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Command {
    pub args: Vec<Arg>,
    pub event: Option<(String, Event)>,
    pub infallible: bool,
    pub optional: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Arg {
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Error {
    pub message: String,
}

impl<'de> Deserialize<'de> for Aggregate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct NamedCommand {
            args: Vec<Arg>,
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
                let mut named_commands: Option<BTreeMap<String, NamedCommand>> = None;
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
                        _ => todo!(),
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let named_commands =
                    named_commands.ok_or_else(|| de::Error::missing_field("commands"))?;
                let events: BTreeMap<String, Event> =
                    events.ok_or_else(|| de::Error::missing_field("events"))?;
                let errors = errors.ok_or_else(|| de::Error::missing_field("errors"))?;

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
                                args: command.args,
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

// #[cfg(test)]
// mod tests {

//     use std::collections::BTreeMap;

//     use crate::schema::{Aggregate, Arg, Command, Error, Event, Field};

//     #[test]
//     fn it_deserializes_correctly() -> Result<(), serde_yaml::Error> {
//         let config = r#"
//         # The name of the aggregate
//         name: BankAccount

//         # The events resulted resulting from commands
//         events:
//           AccountOpened:
//             fields:
//               - name: initial_balance
//                 type: f64

//           DepositedFunds:
//             fields:
//               - name: amount
//                 type: f64

//           WithdrewFunds:
//             fields:
//               - name: amount
//                 type: f64

//         # The commands availble for the aggregate
//         commands:
//           open_account:
//             event: AccountOpened # resulting event when command is successful
//             args:
//               - name: initial_balance
//                 type: f64

//           deposit_funds:
//             event: DepositedFunds
//             args:
//               - name: amount
//                 type: f64

//           withdraw_funds:
//             event: WithdrewFunds
//             args:
//               - name: amount
//                 type: f64

//         # Errors that can occur when executing a command
//         errors:
//           NegativeAmount:
//             message: "amount cannot be negative"

//           InsufficientFunds:
//             message: "insufficient funds for transaction"
//         "#;

//         let aggregate = serde_yaml::from_str::<Aggregate>(config)?;

//         let name = "BankAccount".to_string();

//         let mut events = BTreeMap::new();
//         let account_opened = Event {
//             fields: vec![Field {
//                 name: "initial_balance".to_string(),
//                 field_type: "f64".to_string(),
//             }],
//         };
//         let deposited_funds = Event {
//             fields: vec![Field {
//                 name: "amount".to_string(),
//                 field_type: "f64".to_string(),
//             }],
//         };
//         let withdrew_funds = Event {
//             fields: vec![Field {
//                 name: "amount".to_string(),
//                 field_type: "f64".to_string(),
//             }],
//         };
//         events.insert("AccountOpened".to_string(), account_opened.clone());
//         events.insert("DepositedFunds".to_string(), deposited_funds.clone());
//         events.insert("WithdrewFunds".to_string(), withdrew_funds.clone());

//         let mut commands = BTreeMap::new();
//         commands.insert(
//             "open_account".to_string(),
//             Command {
//                 event: ("AccountOpened".to_string(), account_opened),
//                 args: vec![Arg {
//                     name: "initial_balance".to_string(),
//                     type_field: "f64".to_string(),
//                 }],
//             },
//         );
//         commands.insert(
//             "deposit_funds".to_string(),
//             Command {
//                 event: ("DepositedFunds".to_string(), deposited_funds),
//                 args: vec![Arg {
//                     name: "amount".to_string(),
//                     type_field: "f64".to_string(),
//                 }],
//             },
//         );
//         commands.insert(
//             "withdraw_funds".to_string(),
//             Command {
//                 event: ("WithdrewFunds".to_string(), withdrew_funds),
//                 args: vec![Arg {
//                     name: "amount".to_string(),
//                     type_field: "f64".to_string(),
//                 }],
//             },
//         );

//         let mut errors = BTreeMap::new();
//         errors.insert(
//             "NegativeAmount".to_string(),
//             Error {
//                 message: "amount cannot be negative".to_string(),
//             },
//         );
//         errors.insert(
//             "InsufficientFunds".to_string(),
//             Error {
//                 message: "insufficient funds for transaction".to_string(),
//             },
//         );

//         assert_eq!(
//             aggregate,
//             Aggregate {
//                 name,
//                 events,
//                 commands,
//                 errors,
//             }
//         );

//         Ok(())
//     }
// }
