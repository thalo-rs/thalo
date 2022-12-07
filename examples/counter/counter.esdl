version = "0.1.1"

aggregate Counter {
  increment(amount: Int) -> Incremented
  decrement(amount: Int) -> Decremented
}

event Incremented {
  amount: Int
  count: Long
}

event Decremented {
  amount: Int
  count: Long
}
