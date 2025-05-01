use csv::ReaderBuilder;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;

mod account;
mod transaction;

use account::ClientAccount;
use transaction::Transaction;

pub struct PaymentSystem {
    accounts: HashMap<u16, ClientAccount>,
    transactions: HashMap<u32, Transaction>,
}

impl Default for PaymentSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentSystem {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    pub fn process_transaction(&mut self, tx: &Transaction) -> Result<(), String> {
        if (tx.tx_type == *"deposit" || tx.tx_type == *"withdraw")
            && self.transactions.contains_key(&tx.tx)
        {
            return Err("Transaction already processed".to_string());
        }

        let account = self
            .accounts
            .entry(tx.client)
            .or_insert(ClientAccount::new(tx.client));

        if account.locked {
            return Err("Account is locked".to_string());
        }

        match tx.tx_type.as_str() {
            "deposit" => {
                if let Some(amount) = tx.amount {
                    let rounded_amount = amount.round_dp(4);

                    account.available += rounded_amount;
                    account.total += rounded_amount;

                    self.transactions.insert(tx.tx, tx.clone());
                }
            }
            "withdraw" => {
                if let Some(amount) = tx.amount {
                    if account.available >= amount {
                        let rounded_amount = amount.round_dp(4);
                        account.available -= rounded_amount;
                        account.total -= rounded_amount;

                        self.transactions.insert(tx.tx, tx.clone());
                    } else {
                        return Err("Insufficient funds".to_string());
                    }
                }
            }
            "dispute" => {
                if let Some(prev_tx) = self.transactions.get(&tx.tx) {
                    if prev_tx.tx_type.as_str() == "deposit" {
                        if let Some(amount) = prev_tx.amount {
                            let rounded_amount = amount.round_dp(4);
                            account.available -= rounded_amount;
                            account.held += rounded_amount;
                        } else {
                            return Err("Incorrect transaction disputed".to_string());
                        }
                    }
                }
            }
            "resolve" => {
                if let Some(prev_tx) = self.transactions.get(&tx.tx) {
                    if prev_tx.tx_type.as_str() == "deposit" {
                        if let Some(amount) = prev_tx.amount {
                            if account.held >= amount {
                                let rounded_amount = amount.round_dp(4);
                                account.held -= rounded_amount;
                                account.available += rounded_amount;
                            }
                        } else {
                            return Err("Incorrect transaction resolution".to_string());
                        }
                    }
                }
            }
            "chargeback" => {
                if let Some(prev_tx) = self.transactions.get(&tx.tx) {
                    if prev_tx.tx_type.as_str() == "deposit" {
                        if let Some(amount) = prev_tx.amount {
                            let rounded_amount = amount.round_dp(4);
                            account.total -= rounded_amount;
                            account.held -= rounded_amount;
                            account.locked = true;
                        } else {
                            return Err("Incorrect transaction chargeback".to_string());
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut payment_system = PaymentSystem::new();

    let args: Vec<String> = env::args().collect();
    let input_file = &args[1];

    let file = File::open(input_file)?;
    let mut rdr = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .has_headers(true)
        .from_reader(file);

    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(["client", "available", "held", "total", "locked"])?;

    for result in rdr.deserialize() {
        let tx: Transaction = result?;
        payment_system.process_transaction(&tx)?;
    }

    for account in payment_system.accounts.values() {
        wtr.serialize(&[
            account.client.to_string(),
            format!("{:.4}", account.available),
            format!("{:.4}", account.held),
            format!("{:.4}", account.total),
            account.locked.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{PaymentSystem, account::ClientAccount, transaction::Transaction};
    use rust_decimal_macros::dec;

    #[test]
    fn it_correctly_creates_the_payment_system() {
        let payment_system = PaymentSystem::new();

        assert_eq!(payment_system.accounts.len(), 0);
        assert_eq!(payment_system.transactions.len(), 0);
    }

    #[test]
    fn it_correctly_processes_a_deposit() {
        let mut payment_system = PaymentSystem::new();

        let test_tx = Transaction {
            tx_type: "deposit".to_string(),
            client: 0,
            tx: 0,
            amount: Some(dec!(100.0)),
        };

        let _ = payment_system.process_transaction(&test_tx);

        assert_eq!(payment_system.accounts.get(&0).is_some(), true);
        let account: &ClientAccount = payment_system.accounts.get(&0).unwrap();
        assert_eq!(account.available, dec!(100));
        assert_eq!(account.total, dec!(100));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.locked, false);
    }
    #[test]
    fn it_correctly_processes_a_withdrawal_on_an_account_with_balance() {
        let mut payment_system = PaymentSystem::new();

        let test_deposit_tx = Transaction {
            tx_type: "deposit".to_string(),
            client: 0,
            tx: 0,
            amount: Some(dec!(100.0)),
        };

        let _ = payment_system.process_transaction(&test_deposit_tx);

        let test_withdraw_tx = Transaction {
            tx_type: "withdraw".to_string(),
            client: 0,
            tx: 1,
            amount: Some(dec!(50.0)),
        };
        let _ = payment_system.process_transaction(&test_withdraw_tx);
        let account: &ClientAccount = payment_system.accounts.get(&0).unwrap();

        assert_eq!(account.available, dec!(50));
        assert_eq!(account.total, dec!(50));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.locked, false);
    }
    #[test]
    fn it_correctly_processes_a_dispute() {
        let mut payment_system = PaymentSystem::new();

        let test_deposit_tx = Transaction {
            tx_type: "deposit".to_string(),
            client: 0,
            tx: 0,
            amount: Some(dec!(100.0)),
        };

        let _ = payment_system.process_transaction(&test_deposit_tx);

        let test_dispute_tx = Transaction {
            tx_type: "dispute".to_string(),
            client: 0,
            tx: 0,
            amount: None,
        };

        let _ = payment_system.process_transaction(&test_dispute_tx);
        let account: &ClientAccount = payment_system.accounts.get(&0).unwrap();

        assert_eq!(account.total, dec!(100));
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(100));
        assert_eq!(account.locked, false);
    }

    #[test]
    fn it_correctly_processes_dispute_resolutions() {
        let mut payment_system = PaymentSystem::new();

        let test_deposit_tx = Transaction {
            tx_type: "deposit".to_string(),
            client: 0,
            tx: 0,
            amount: Some(dec!(100.0)),
        };

        let _ = payment_system.process_transaction(&test_deposit_tx);

        let test_dispute_tx = Transaction {
            tx_type: "dispute".to_string(),
            client: 0,
            tx: 0,
            amount: None,
        };

        let _ = payment_system.process_transaction(&test_dispute_tx);

        let test_resolve_tx = Transaction {
            tx_type: "resolve".to_string(),
            client: 0,
            tx: 0,
            amount: None,
        };

        let _ = payment_system.process_transaction(&test_resolve_tx);

        let account: &ClientAccount = payment_system.accounts.get(&0).unwrap();
        assert_eq!(account.total, dec!(100));
        assert_eq!(account.available, dec!(100));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.locked, false);
    }

    #[test]
    fn it_correctly_processes_a_chargeback() {
        let mut payment_system = PaymentSystem::new();

        let test_deposit_tx = Transaction {
            tx_type: "deposit".to_string(),
            client: 0,
            tx: 0,
            amount: Some(dec!(100.0)),
        };

        let _ = payment_system.process_transaction(&test_deposit_tx);

        let test_dispute_tx = Transaction {
            tx_type: "dispute".to_string(),
            client: 0,
            tx: 0,
            amount: None,
        };

        let _ = payment_system.process_transaction(&test_dispute_tx);

        let test_chargeback_tx = Transaction {
            tx_type: "chargeback".to_string(),
            client: 0,
            tx: 0,
            amount: None,
        };
        let _ = payment_system.process_transaction(&test_chargeback_tx);
        let account: &ClientAccount = payment_system.accounts.get(&0).unwrap();

        assert_eq!(account.available, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.locked, true);
    }

    #[test]
    fn it_correctly_returns_an_error_when_withdrawing_with_insufficent_funds() {
        let mut payment_system = PaymentSystem::new();
        let test_withdraw_tx = Transaction {
            tx_type: "withdraw".to_string(),
            client: 0,
            tx: 0,
            amount: Some(dec!(50.0)),
        };
        assert_eq!(
            payment_system.process_transaction(&test_withdraw_tx),
            Err("Insufficient funds".to_string())
        );
    }
}
