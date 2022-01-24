aggregate BankAccount {
  open_account(initial_balance: Float!): OpenedAccount!
  deposit_funds(amount: Float!): DepositedFunds!
  withdraw_funds(amount: Float!): WithdrewFunds!
}

event OpenedAccount {
  initial_balance: Float!
}

event DepositedFunds {
  amount: Float!
}

event WithdrewFunds {
  amount: Float!
}
