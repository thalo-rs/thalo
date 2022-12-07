version = "0.1.0"

aggregate Todos {
  add(id: String, description: String) -> Added
  remove(id: String) -> Removed
}

event Added {
  id: String
  description: String
}

event Removed {
  id: String
}
