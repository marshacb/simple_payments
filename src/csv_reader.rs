use crate::{SharedPaymentSystem, Transaction};
use csv::ReaderBuilder;
use csv::Writer;
use std::io;
use std::io::Read;
use std::sync::mpsc;

pub fn process_csv<R: Read>(
    reader: R,
    payment_system: SharedPaymentSystem,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .has_headers(true)
        .from_reader(reader);

    let mut wtr = Writer::from_writer(io::stdout());
    wtr.write_record(["client", "available", "held", "total", "locked"])?;

    let mut payment_system_lock = payment_system.lock().unwrap();

    for result in rdr.deserialize() {
        let tx: Transaction = result?;
        let _ = payment_system_lock.process_transaction(&tx).map_err(|e| {
            eprintln!("client: {}, tx: {:?} error: {}", tx.client, tx.tx_type, e);
        });
    }

    for account in payment_system_lock.accounts.values() {
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

pub fn process_csv_stream<R: Read>(
    reader: R,
    payment_system: SharedPaymentSystem,
    tx: mpsc::Sender<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .has_headers(true)
        .from_reader(reader);

    for result in rdr.deserialize() {
        let tx_record: Transaction = result?;
        let mut payment_system_lock = payment_system.lock().unwrap();

        if let Err(e) = payment_system_lock.process_transaction(&tx_record) {
            eprintln!("failed to process transaction: {}", e);
            continue;
        }

        if let Some(account) = payment_system_lock.accounts.get(&tx_record.client) {
            tx.send(vec![
                account.client.to_string(),
                format!("{:.4}", account.available),
                format!("{:.4}", account.held),
                format!("{:.4}", account.total),
                account.locked.to_string(),
            ])?;
        }
    }

    Ok(())
}
