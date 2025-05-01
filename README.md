# Payment Transaction Processor

This project implements a payment transaction engine in Rust that reads financial transaction records in CSV format and maintains accurate account balances per client.

It supports two modes:

- **File mode**: Reads a CSV input from a file (or stdin) and writes processed account states to stdout or a file.
- **TCP mode (`--tcp`)**: Accepts CSV streams from multiple concurrent TCP clients and processes them in parallel.

---

## Features

- Supports transaction types:
  - `deposit`
  - `withdrawal`
  - `dispute`
  - `resolve`
  - `chargeback`
- Accurate floating-point math using [`rust_decimal`](https://docs.rs/rust_decimal/)
- Graceful handling of malformed or duplicate transactions
- Output format:
  - `client`, `available`, `held`, `total`, `locked`
- Unit tested for correctness
- TCP mode using multithreading and channels for concurrent stream handling

---

## Usage

### 📄 File Mode (Default)

Process a CSV file and write the resulting account states:

```bash
cargo run -- input.csv > output.csv
```

### 🌐 TCP Mode (Concurrent Streaming)

Start the engine in TCP server mode:

```bash
cargo run -- --tcp
```

It listens on `127.0.0.1:4000` and processes multiple TCP streams in parallel. Each stream should send valid CSV data in the same format as file mode. The results are written to a single `output.csv` file.

> **Note**: This mode is intended as a proof-of-concept for real-time streaming input from distributed sources.

---

## CSV Format

Each row should represent a transaction:

```csv
type, client, tx, amount
deposit, 1, 1, 1.0
withdrawal, 1, 2, 0.5
dispute, 1, 1
resolve, 1, 1
chargeback, 1, 1
```

- The `amount` field is only required for `deposit` and `withdrawal`.
- Fields must match the expected column count per row, or a parse error will occur.

---

## Example Output

```csv
client,available,held,total,locked
1,0.5000,0.0000,0.5000,false
```

---

## Architecture

- `PaymentSystem`: Core state manager, holds account and transaction ledgers.
- `Arc<Mutex<...>>`: Shared state safely accessible from multiple threads.
- `csv_reader`: Contains logic for reading and parsing CSV input from file or stream.
- TCP mode uses:
  - `TcpListener` to accept clients
  - `std::thread` to spawn stream workers
  - `mpsc::channel` to send processed output to a writer thread

---

## Development

Build and run tests:

```bash
cargo build
cargo test
```
