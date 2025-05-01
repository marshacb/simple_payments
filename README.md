# 🏦 Payment Processing System

A performant and robust command-line tool to process and manage client transactions using Rust. The system supports deposits, withdrawals, disputes, resolutions, and chargebacks, handling CSV input and producing accurate, rounded output.

---

## 📋 Features

- Supports transaction types: `deposit`, `withdraw`, `dispute`, `resolve`, `chargeback`
- Ensures 4-decimal place precision using `rust_decimal`
- Maintains accurate client account states (available, held, total, locked)
- Prevents duplicate processing and handles account locking
- Written with safe streaming and in-memory efficiency
- Includes a comprehensive test suite

---

## 🚀 Getting Started

### Requirements

- Rust (edition 2021 recommended)
- Cargo (Rust package manager)

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run --release -- path/to/input.csv > output.csv
```

### Input CSV Format

CSV should include headers and follow this structure:

```csv
type, client, tx, amount
deposit, 1, 1, 100.0
withdraw, 1, 2, 50.0
```

Supported `type` values:

- `deposit`
- `withdraw`
- `dispute`
- `resolve`
- `chargeback`

### Output CSV Format

After processing, the program prints to `stdout`:

```csv
client,available,held,total,locked
1,50.0000,0.0000,50.0000,false
```

---

## 🧪 Running Tests

```bash
cargo test
```

---

## 🗃 Project Structure

- `main.rs` – Entry point and main CSV processing logic
- `account.rs` – Client account model and logic
- `transaction.rs` – Transaction data model
- `tests` – Extensive unit tests for transaction logic

---

## 📦 Dependencies

- [`csv`](https://docs.rs/csv) – Fast CSV reading/writing
- [`rust_decimal`](https://docs.rs/rust_decimal) – Precise decimal arithmetic
