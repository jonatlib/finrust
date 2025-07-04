# Transaction Processing Documentation

This document describes how transactions are processed in the compute workspace of the FinRust project.

## Transaction to DataFrame Conversion

The `transaction.rs` file in the compute workspace provides functionality for converting Transaction objects to Polars DataFrames for analysis:

1. `TransactionPolars` trait: Extends the Transaction model with methods to convert to Polars Series and DataFrames
2. `TransactionIteratorPolars` trait: Extends iterators over Transactions with methods to convert to DataFrames
3. Helper functions like `transactions_to_df` for convenient conversion

## Data Representation

Transactions are represented in DataFrames with the following columns:

- `date`: The transaction date as a timestamp (milliseconds since epoch)
- `amount`: The transaction amount as a floating-point number
- `account`: The account ID as an integer

## Usage Examples

### Converting a Single Transaction

```
let transaction = Transaction::new(date, amount, account);
let df = transaction.to_df().unwrap();
```

### Converting Multiple Transactions

```
let transactions: Vec<Transaction> = /* ... */;
let df = transactions_to_df(&transactions).unwrap();
```

### Working with Transaction Iterators

```
let transactions_iter = some_generator.generate_transactions(start_date, end_date);
let df = transactions_iter.to_df().unwrap();
```

## Integration with Account Calculations

The transaction processing functionality integrates with the account calculation system:

1. Transactions are generated using the `TransactionGenerator` trait
2. Transactions are converted to DataFrames using the `TransactionPolars` trait
3. DataFrames are processed using Polars operations for financial calculations
4. Results are merged using the `MergeCalculator` based on the specified merge method

## Performance Considerations

- The implementation efficiently handles large transaction sets by collecting all data before creating the DataFrame
- For very large datasets, consider processing transactions in batches