# Model Workspace

This workspace contains the data models and entity definitions for the FinRust project, providing the core data structures used throughout the application.

## Overview

The model workspace defines:

- Transaction data structures
- Entity models for various financial objects
- Traits for generating and working with transactions

## Structure

- `src/entities/`: Contains various entity models including:
  - `imported_transaction/`: Models for imported transactions
  - `one_off_transaction/`: Models for one-off transactions
- `src/transaction.rs`: Core Transaction struct and TransactionGenerator trait
- `src/lib.rs`: Main library entry point exporting the modules

## Key Components

### Transaction

The `Transaction` struct represents a single financial transaction with a date, amount, and account ID. It provides methods for accessing these properties.

### TransactionGenerator

The `TransactionGenerator` trait defines an interface for types that can generate transactions within a date range:

```
pub trait TransactionGenerator {
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool;
    fn generate_transactions(&self, start: NaiveDate, end: NaiveDate) -> impl Iterator<Item = Transaction>;
}
```

## Usage

The models defined in this workspace are used by the compute workspace and other parts of the application to perform financial calculations and analysis.