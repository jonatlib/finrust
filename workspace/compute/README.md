# Compute Workspace

This workspace contains the computational components of the FinRust project, focusing on financial calculations, data transformations, and analysis.

## Overview

The compute workspace provides functionality for:

- Converting financial transaction models to Polars DataFrames for analysis
- Calculating account balances
- Processing recurring transactions
- Merging different calculation methods

## Structure

- `src/account/`: Account-related calculations including balance, merge, and unpaid recurring transactions
- `src/transaction.rs`: Utilities for converting Transaction objects to Polars DataFrames
- `src/error.rs`: Error handling for the compute operations
- `src/lib.rs`: Main library entry point with default compute configuration

## Usage

The main entry point is the `default_compute` function in `lib.rs`, which provides a pre-configured compute instance with balance and unpaid recurring calculators.

Example:

```
use compute::default_compute;

// Use current date
let calculator = default_compute(None);

// Or specify a date
let specific_date = chrono::NaiveDate::from_ymd_opt(2023, 1, 1);
let calculator = default_compute(specific_date);
```

## Documentation

Additional documentation and TODOs can be found in the `docs/` directory.