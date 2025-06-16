# FinRust - Personal Finance Management System

## Project Overview
FinRust is a comprehensive personal finance management system designed to help users track, analyze, and manage their financial activities. The system allows for detailed tracking of accounts, transactions, expenses, and financial states over time.

## Architecture
The project is structured as a Cargo workspace with distinct crates for different concerns:

- **shared**: Contains data transfer objects (DTOs) that define the public API contract
- **core**: Contains the business logic and database entities
- **api**: Implements the HTTP server and API endpoints using Axum
- **cli**: Provides a command-line interface for data import and export

This architecture ensures a clean separation of concerns and allows for code reuse across different interfaces.

## Core Features

### Account Management
- Create and manage multiple money accounts with different currencies
- Track account balances over time
- Support for manual account state snapshots to reconcile with real-world balances
- User permissions system to control access to accounts

### Transaction Tracking
- Record both regular (recurring) and one-time transactions
- Support for different transaction periods (daily, weekly, monthly, yearly)
- Detailed transaction metadata including dates, amounts, descriptions
- Transaction categorization with tags and categories

### Financial Analysis
- Generate expense reports by category and tag
- View expenses aggregated by month
- Track account balances over time
- Visualize financial data with charts and graphs
- Compare real vs. expected account balances

### Categorization System
- Hierarchical category structure for organizing transactions
- Tag system for flexible transaction labeling
- Color-coding for visual organization
- Group transactions by various attributes for analysis

### Accounting Features
- Double-entry accounting system
- Ledger export functionality for accounting purposes
- Support for different currencies with proper formatting
- Historical record tracking for all financial data

### Reporting and Visualization
- Interactive charts and graphs for financial data
- Balance history visualization
- Expense breakdown by category and tag
- Monthly expense trends

### Import/Export
- Support for importing data from banking exports (CSV, OFX)
- Deduplication of imported transactions
- Export to Ledger file format for compatibility with other tools

## Technical Capabilities
- Data processing and aggregation
- Time-series financial data handling
- Currency formatting and conversion using rusty_money
- Date-based transaction generation for recurring expenses
- Historical data tracking and versioning

## Getting Started

### Prerequisites
- Rust (latest stable version)
- PostgreSQL database

### Setup
1. Clone the repository
2. Set up the database:
   ```bash
   createdb finrust
   ```
3. Set the database URL environment variable:
   ```bash
   export DATABASE_URL=postgres://postgres:postgres@localhost/finrust
   ```
4. Build the project:
   ```bash
   cargo build --release
   ```

### Running the API Server
```bash
cargo run --release -p api
```

The API server will start on http://localhost:3000.

### Using the CLI
Export transactions to Ledger format:
```bash
cargo run --release -p cli -- export --output transactions.ledger
```

Import transactions from a CSV file:
```bash
cargo run --release -p cli -- import --input bank_export.csv --account 1
```

## API Endpoints

### Transactions
- `GET /api/transactions/:id` - Get a single transaction
- `GET /api/transactions` - Get all transactions
- `POST /api/transactions` - Create a new transaction
- `GET /api/accounts/:id/transactions` - Get transactions for a specific account

## Development

### Running Tests
```bash
cargo test
```

### Project Structure
```
finrust/
├── api/               # Web API implementation
├── cli/               # Command-line interface
├── core/              # Business logic and database entities
├── shared/            # Shared data models
└── Cargo.toml         # Workspace definition
```

## Design Decisions

### Why SeaORM?
We chose SeaORM as our primary ORM because it is async-native, integrates perfectly with Axum/Tokio, and offers a productive Active Record pattern for standard CRUD operations. Its migration tooling is also written in Rust, providing a unified experience.

### Why rusty_money?
For all internal logic, monetary values must be handled with a precise decimal type to prevent rounding errors. rusty_money is the ideal library for this, providing both the Money type and currency information.

### Why Axum?
Axum is our framework of choice because it is built by the Tokio team, guaranteeing stability and seamless integration with the async ecosystem. Its use of the tower service model provides a powerful and composable way to add middleware for concerns like logging, authentication, and error handling.

### Why Clap?
Clap is the de-facto standard for building powerful and ergonomic CLIs in Rust. Its derive-based API makes it incredibly easy to define complex subcommands and arguments, and it automatically generates helpful --help messages.

## Future Enhancements
- User authentication and authorization
- Mobile app integration
- Budgeting features
- Investment tracking
- Tax reporting
- Multi-user support
- Notifications for financial events

## License
This project is licensed under the MIT License - see the LICENSE file for details.