#![allow(unused_must_use)]

use std::{collections::BTreeMap, env, fmt::Write, fs, path::Path};

use crate::{
    schema::{self, Aggregate, Command, Event},
    Error,
};

#[derive(Default)]
pub struct Compiler {
    schemas: Vec<Aggregate>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            schemas: Vec::new(),
        }
    }

    pub fn add_schema(mut self, schema: Aggregate) -> Self {
        self.schemas.push(schema);
        self
    }

    pub fn add_schema_file<P: AsRef<Path>>(self, path: P) -> Result<Self, Error> {
        let content = fs::read(path)?;
        let aggregate: Aggregate = serde_yaml::from_slice(&content)?;
        Ok(self.add_schema(aggregate))
    }

    pub fn add_schema_str(self, content: &str) -> Result<Self, Error> {
        let aggregate: Aggregate = serde_yaml::from_str(content)?;
        Ok(self.add_schema(aggregate))
    }

    pub fn compile(self) -> Result<(), Error> {
        let out_dir = env::var("OUT_DIR").unwrap();

        for schema in self.schemas {
            let code = Self::compile_schema(&schema);
            fs::write(format!("{}/{}.rs", out_dir, schema.name), code)?;
        }

        Ok(())
    }

    fn compile_schema(schema: &Aggregate) -> String {
        let mut code = String::new();

        Self::compile_schema_events(&mut code, &schema.name, &schema.events);

        Self::compile_schema_errors(&mut code, &schema.name, &schema.errors);

        Self::compile_schema_commands(&mut code, &schema.name, &schema.commands);

        code
    }

    fn compile_schema_events(code: &mut String, name: &str, events: &BTreeMap<String, Event>) {
        writeln!(
            code,
            "#[derive(Clone, Debug, serde::Deserialize, thalo::event::EventType, PartialEq, serde::Serialize)]"
        );
        writeln!(code, "pub enum {}Event {{", name);

        for event_name in events.keys() {
            writeln!(code, "    {}({0}Event),", event_name);
        }

        writeln!(code, "}}\n");

        for (event_name, event) in events {
            writeln!(code, "#[derive(Clone, Debug, serde::Deserialize, thalo::event::Event, PartialEq, serde::Serialize)]");
            writeln!(
                code,
                r#"#[thalo(parent = "{}Event", variant = "{}")]"#,
                name, event_name
            );
            writeln!(code, "pub struct {}Event {{", event_name);
            for field in &event.fields {
                writeln!(code, "    pub {}: {},", field.name, field.field_type);
            }
            writeln!(code, "}}\n");
        }
    }

    fn compile_schema_errors(
        code: &mut String,
        name: &str,
        errors: &BTreeMap<String, schema::Error>,
    ) {
        writeln!(code, "#[derive(Clone, Debug, thiserror::Error, PartialEq)]");
        writeln!(code, "pub enum {}Error {{", name);

        for (error_name, error) in errors {
            writeln!(code, r#"    #[error("{}")]"#, error.message);
            writeln!(code, "    {},", error_name);
        }

        writeln!(code, "}}");
    }

    fn compile_schema_commands(
        code: &mut String,
        name: &str,
        commands: &BTreeMap<String, Command>,
    ) {
        writeln!(code, "pub trait {}Command {{", name);

        for (command_name, command) in commands {
            write!(code, "    fn {}(&self", command_name);

            for arg in &command.args {
                write!(code, ", {}: {}", arg.name, arg.type_field);
            }

            write!(code, ") -> ");

            if !command.infallible {
                write!(code, "std::result::Result<");
            }

            if command.event.is_some() && command.optional {
                write!(code, "std::option::Option<");
            }

            match &command.event {
                Some((event_name, _)) => {
                    write!(code, "{}Event", event_name);
                }
                None => {
                    write!(code, "Vec<{}Event>", name);
                }
            }

            if command.event.is_some() && command.optional {
                write!(code, ">");
            }

            if !command.infallible {
                write!(code, ", {}Error>", name);
            }

            // pub fn extend_refresh_token(&self, now: DateTime<Utc>) -> Result<Option<ExtendedRefreshTokenEvent>, AuthError> {
            writeln!(code, ";");
        }

        writeln!(code, "}}");
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use crate::{
        schema::{Aggregate, Event, Field},
        Compiler,
    };

    #[test]
    fn it_compile_schema_events() {
        let mut events = BTreeMap::new();
        events.insert(
            "AccountOpened".to_string(),
            Event {
                fields: vec![Field {
                    name: "initial_balance".to_string(),
                    field_type: "f64".to_string(),
                }],
            },
        );
        events.insert(
            "DepositedFunds".to_string(),
            Event {
                fields: vec![Field {
                    name: "amount".to_string(),
                    field_type: "f64".to_string(),
                }],
            },
        );
        events.insert(
            "WithdrewFunds".to_string(),
            Event {
                fields: vec![Field {
                    name: "amount".to_string(),
                    field_type: "f64".to_string(),
                }],
            },
        );

        let mut code = String::new();
        Compiler::compile_schema_events(&mut code, "BankAccount", &events);

        assert_eq!(
            code,
            r#"#[derive(Clone, Debug, serde::Deserialize, thalo::event::EventType, PartialEq, serde::Serialize)]
pub enum BankAccountEvent {
    AccountOpened(AccountOpenedEvent),
    DepositedFunds(DepositedFundsEvent),
    WithdrewFunds(WithdrewFundsEvent),
}

#[derive(Clone, Debug, serde::Deserialize, thalo::event::Event, PartialEq, serde::Serialize)]
#[thalo(parent = "BankAccountEvent", variant = "AccountOpened")]
pub struct AccountOpenedEvent {
    pub initial_balance: f64,
}

#[derive(Clone, Debug, serde::Deserialize, thalo::event::Event, PartialEq, serde::Serialize)]
#[thalo(parent = "BankAccountEvent", variant = "DepositedFunds")]
pub struct DepositedFundsEvent {
    pub amount: f64,
}

#[derive(Clone, Debug, serde::Deserialize, thalo::event::Event, PartialEq, serde::Serialize)]
#[thalo(parent = "BankAccountEvent", variant = "WithdrewFunds")]
pub struct WithdrewFundsEvent {
    pub amount: f64,
}

"#
        )
    }

    #[test]
    fn it_compile_schema_commands() {
        let config = fs::read_to_string("./bank-account.yaml").unwrap();
        let schema: Aggregate = serde_yaml::from_str(&config).unwrap();

        let mut code = String::new();
        Compiler::compile_schema_commands(&mut code, &schema.name, &schema.commands);

        println!("{}", code);
    }
}
