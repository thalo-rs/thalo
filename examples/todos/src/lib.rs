#![allow(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thalo::{export_aggregate, Aggregate, Events};

export_aggregate!(Todos);

pub struct Todos {
    id: String,
    todos: HashMap<String, Todo>,
}

struct Todo {
    description: String,
}

impl Aggregate for Todos {
    type ID = String;
    type Command = TodosCommand;
    type Events = TodosEvents;
    type Error = &'static str;

    fn init(id: Self::ID) -> Self {
        Todos {
            id,
            todos: HashMap::new(),
        }
    }

    fn apply(&mut self, event: <Self::Events as Events>::Event) {
        use Event::*;

        match event {
            TodoAdded(TodoAddedV1 { id, description }) => {
                self.todos.insert(id, Todo { description });
            }
            TodoRemoved(TodoRemovedV1 { id }) => {
                self.todos.remove(&id);
            }
        }
    }

    fn handle(
        &self,
        cmd: Self::Command,
    ) -> Result<Vec<<Self::Events as Events>::Event>, Self::Error> {
        use Event::*;
        use TodosCommand::*;

        match cmd {
            AddTodo { id, description } => {
                if self.todos.contains_key(&id) {
                    return Err("todo already added");
                }

                Ok(vec![TodoAdded(TodoAddedV1 { id, description })])
            }
            RemoveTodo { id } => Ok(vec![TodoRemoved(TodoRemovedV1 { id })]),
        }
    }
}

#[derive(Deserialize)]
pub enum TodosCommand {
    AddTodo { id: String, description: String },
    RemoveTodo { id: String },
}

#[derive(Events)]
pub enum TodosEvents {
    TodoAdded(TodoAddedV1),
    TodoRemoved(TodoRemovedV1),
}

#[derive(Serialize, Deserialize)]
pub struct TodoAddedV1 {
    id: String,
    description: String,
}

#[derive(Serialize, Deserialize)]
pub struct TodoRemovedV1 {
    id: String,
}
