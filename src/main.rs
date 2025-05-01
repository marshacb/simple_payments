use csv::Writer;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::net::TcpListener;
use std::process;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
mod account;
mod csv_reader;
mod transaction;

use account::ClientAccount;
use transaction::{Transaction, TransactionType};

type SharedPaymentSystem = Arc<Mutex<PaymentSystem>>;

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
        if (tx.tx_type == TransactionType::Deposit || tx.tx_type == TransactionType::Withdraw)
            && self.transactions.contains_key(&tx.tx)
        {
            return Err("transaction already processed".to_string());
        }

        let account = self
            .accounts
            .entry(tx.client)
            .or_insert(ClientAccount::new(tx.client));

        if account.locked {
            return Err("account is locked".to_string());
        }

        match tx.tx_type {
            TransactionType::Deposit => {
                if let Some(amount) = tx.amount {
                    let rounded_amount = amount.round_dp(4);

                    account.available += rounded_amount;
                    account.total += rounded_amount;

                    self.transactions.insert(tx.tx, tx.clone());
                }
            }
            TransactionType::Withdraw => {
                if let Some(amount) = tx.amount {
                    if account.available >= amount {
                        let rounded_amount = amount.round_dp(4);
                        account.available -= rounded_amount;
                        account.total -= rounded_amount;

                        self.transactions.insert(tx.tx, tx.clone());
                    } else {
                        return Err("insufficient funds".to_string());
                    }
                }
            }
            TransactionType::Dispute => {
                if let Some(prev_tx) = self.transactions.get(&tx.tx) {
                    if prev_tx.tx_type == TransactionType::Deposit {
                        if let Some(amount) = prev_tx.amount {
                            let rounded_amount = amount.round_dp(4);
                            account.available -= rounded_amount;
                            account.held += rounded_amount;
                        }
                    }
                }
            }
            TransactionType::Resolve => {
                if let Some(prev_tx) = self.transactions.get(&tx.tx) {
                    if prev_tx.tx_type == TransactionType::Deposit {
                        if let Some(amount) = prev_tx.amount {
                            if account.held >= amount {
                                let rounded_amount = amount.round_dp(4);
                                account.held -= rounded_amount;
                                account.available += rounded_amount;
                            }
                        }
                    }
                }
            }
            TransactionType::Chargeback => {
                if let Some(prev_tx) = self.transactions.get(&tx.tx) {
                    if prev_tx.tx_type == TransactionType::Deposit {
                        if let Some(amount) = prev_tx.amount {
                            let rounded_amount = amount.round_dp(4);
                            account.total -= rounded_amount;
                            account.held -= rounded_amount;
                            account.locked = true;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let payment_system = Arc::new(Mutex::new(PaymentSystem::new()));
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--tcp".to_string()) {
        println!("Starting in TCP mode...");
        let listener = TcpListener::bind("127.0.0.1:4000")?;

        let (tx, rx) = mpsc::channel::<Vec<String>>();

        let writer_handle = thread::spawn(move || {
            let mut wtr = Writer::from_path("output.csv").expect("Failed to open file");
            wtr.write_record(["client", "available", "held", "total", "locked"])
                .unwrap();
            wtr.flush().unwrap();

            for record in rx {
                if let Err(e) = wtr.write_record(&record) {
                    eprintln!("Failed to write record: {}", e);
                }
                wtr.flush().unwrap();
            }

            wtr.flush().unwrap();
        });

        for stream in listener.incoming() {
            let stream = stream?;
            let ps = Arc::clone(&payment_system);
            let tx = tx.clone();

            thread::spawn(move || {
                csv_reader::process_csv_stream(stream, ps, tx)
                    .unwrap_or_else(|e| eprintln!("Stream error: {}", e));
            });
        }

        drop(tx);
        writer_handle.join().unwrap();

        return Ok(());
    }

    if args.len() == 2 {
        let file_path = &args[1];
        let file = File::open(file_path)?;
        let ps = Arc::clone(&payment_system);
        csv_reader::process_csv(file, ps)?;
    } else {
        eprintln!("Usage:");
        eprintln!("  cargo run -- input_file.csv > output_file.csv");
        eprintln!("  cargo run -- --tcp");
        process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        PaymentSystem,
        account::ClientAccount,
        transaction::{Transaction, TransactionType},
    };
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
            tx_type: TransactionType::Deposit,
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
            tx_type: TransactionType::Deposit,
            client: 0,
            tx: 0,
            amount: Some(dec!(100.0)),
        };

        let _ = payment_system.process_transaction(&test_deposit_tx);

        let test_withdraw_tx = Transaction {
            tx_type: TransactionType::Withdraw,
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
            tx_type: TransactionType::Deposit,
            client: 0,
            tx: 0,
            amount: Some(dec!(100.0)),
        };

        let _ = payment_system.process_transaction(&test_deposit_tx);

        let test_dispute_tx = Transaction {
            tx_type: TransactionType::Dispute,
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
            tx_type: TransactionType::Deposit,
            client: 0,
            tx: 0,
            amount: Some(dec!(100.0)),
        };

        let _ = payment_system.process_transaction(&test_deposit_tx);

        let test_dispute_tx = Transaction {
            tx_type: TransactionType::Dispute,
            client: 0,
            tx: 0,
            amount: None,
        };

        let _ = payment_system.process_transaction(&test_dispute_tx);

        let test_resolve_tx = Transaction {
            tx_type: TransactionType::Resolve,
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
            tx_type: TransactionType::Deposit,
            client: 0,
            tx: 0,
            amount: Some(dec!(100.0)),
        };

        let _ = payment_system.process_transaction(&test_deposit_tx);

        let test_dispute_tx = Transaction {
            tx_type: TransactionType::Dispute,
            client: 0,
            tx: 0,
            amount: None,
        };

        let _ = payment_system.process_transaction(&test_dispute_tx);

        let test_chargeback_tx = Transaction {
            tx_type: TransactionType::Chargeback,
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
            tx_type: TransactionType::Withdraw,
            client: 0,
            tx: 0,
            amount: Some(dec!(50.0)),
        };
        assert_eq!(
            payment_system.process_transaction(&test_withdraw_tx),
            Err("insufficient funds".to_string())
        );
    }
}
