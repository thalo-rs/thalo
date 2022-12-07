version = "0.1.0"

aggregate BankAccount {
  open_account(initial_balance: Long) -> OpenedAccount
  deposit_funds(amount: Int) -> DepositedFunds
  withdraw_funds(amount: Int) -> WithdrewFunds
}

event OpenedAccount {
  initial_balance: Long
}

event DepositedFunds {
  amount: Int
}

event WithdrewFunds {
  amount: Int
}
