use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thalo::{events, export_aggregate, Aggregate, Apply, Command, Event, Handle};
use thiserror::Error;

export_aggregate!(Todos);

pub struct Todos {
    todos: HashMap<String, Todo>,
}

pub struct Todo {
    pub description: String,
}

impl Aggregate for Todos {
    type Command = TodosCommand;
    type Event = TodosEvent;

    fn init(_id: String) -> Self {
        Todos {
            todos: HashMap::new(),
        }
    }
}

#[derive(Command, Deserialize)]
pub enum TodosCommand {
    AddTodo { id: String, description: String },
    RemoveTodo { id: String },
}

#[derive(Debug, Error, Serialize)]
pub enum TodosError {
    #[error("todo already exists")]
    TodoAlreadyExists,
}

impl Handle<TodosCommand> for Todos {
    type Error = TodosError;

    fn handle(&self, cmd: TodosCommand) -> Result<Vec<TodosEvent>, Self::Error> {
        use TodosCommand::*;

        match cmd {
            AddTodo { id, description } => {
                if self.todos.contains_key(&id) {
                    return Err(TodosError::TodoAlreadyExists);
                }

                events![AddedTodo { id, description }]
            }
            RemoveTodo { id } => events![RemovedTodo { id }],
        }
    }
}

#[derive(Event, Serialize, Deserialize)]
pub enum TodosEvent {
    AddedTodo(AddedTodo),
    RemovedTodo(RemovedTodo),
}

#[derive(Serialize, Deserialize)]
pub struct AddedTodo {
    id: String,
    description: String,
}

impl Apply<AddedTodo> for Todos {
    fn apply(&mut self, event: AddedTodo) {
        self.todos.insert(
            event.id,
            Todo {
                description: event.description,
            },
        );
    }
}

#[derive(Serialize, Deserialize)]
pub struct RemovedTodo {
    id: String,
}

impl Apply<RemovedTodo> for Todos {
    fn apply(&mut self, event: RemovedTodo) {
        self.todos.remove(&event.id);
    }
}
