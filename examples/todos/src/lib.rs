use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thalo::{export_aggregate, Aggregate, Context, Error};

export_aggregate!(Todos);

#[derive(Aggregate, Serialize, Deserialize)]
#[aggregate(schema = "examples/todos/todos.esdl")]
pub struct Todos {
    id: String,
    todos: HashMap<String, Todo>,
}

#[derive(Serialize, Deserialize)]
struct Todo {
    description: String,
}

impl TodosAggregate for Todos {
    fn new(id: String) -> Result<Self, thalo::Error> {
        Ok(Todos {
            id,
            todos: HashMap::new(),
        })
    }

    fn apply_added(&mut self, _ctx: Context, event: Added) {
        self.todos.insert(
            event.id,
            Todo {
                description: event.description,
            },
        );
    }

    fn apply_removed(&mut self, _ctx: Context, event: Removed) {
        self.todos.remove(&event.id);
    }

    fn handle_add(
        &self,
        _ctx: &mut Context,
        id: String,
        description: String,
    ) -> Result<Added, Error> {
        if self.todos.contains_key(&id) {
            return Err(Error::fatal("todo already exists"));
        }

        Ok(Added { id, description })
    }

    fn handle_remove(&self, _ctx: &mut Context, id: String) -> Result<Removed, Error> {
        if self.todos.contains_key(&id) {
            return Err(Error::fatal("todo does not exists"));
        }

        Ok(Removed { id })
    }
}
