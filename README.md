# FinRust - Home Finance Tracker

![FinRust Logo](assets/logo-small.png)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

This repository contains the backend service for a powerful, self-hosted home finance tracking application. Built with
Rust, Axum, SeaORM, and Polars, this tool is designed for users who want granular control over their financial data,
robust forecasting capabilities, and a system based on sound accounting principles.

The core mission of this project is to provide a comprehensive and accurate view of your financial situation, both past
and future. It allows you to model your entire financial ecosystem—from various bank accounts and currencies to complex
recurring transactions—and then use that model to gain insights and forecast with precision.

## Table of Contents

- [Core Features](#core-features)
- [Project Goals](#project-goals)
- [Technical Stack](#technical-stack)
- [Getting Started](#getting-started)
    - [Prerequisites](#prerequisites)
    - [Installation](#installation)
    - [Running the Application](#running-the-application)
- [API Documentation](#api-documentation)
    - [Available Endpoints](#available-endpoints)
    - [Interactive Documentation](#interactive-documentation)
- [Project Structure](#project-structure)
- [Development](#development)
    - [Setting Up Development Environment](#setting-up-development-environment)
    - [Running Tests](#running-tests)
- [Contributing](#contributing)
- [License](#license)

## Core Features

This application is built around a set of powerful, interconnected features:

* **Multi-Currency Account Management**:
    * Track an unlimited number of accounts (e.g., bank accounts, credit cards, cash).
    * Each account has its own designated currency, with all calculations being currency-aware using `rusty_money`.
    * Designate specific accounts (e.g., for error correction) to be ignored in statistics and totals.

* **Comprehensive Transaction Modeling**:
    * **Recurring Transactions**: Model regular expenses like rent, subscriptions, or loan payments with flexible
      recurrence rules (daily, weekly, monthly, etc.).
    * **Recurring Income**: Separately model recurring income streams like salaries or business revenue.
    * **One-Off Transactions**: Manually add any extra or non-recurring transactions.
    * **Imported Transactions**: Import transactions from standard banking formats. The system is designed to let you
      reconcile these imported items against your manually modeled data to prevent duplicates.

* **Double-Entry Accounting System**:
    * All transactions support an optional source account in addition to the mandatory target account.
    * When both are specified, the system automatically creates the corresponding transaction on the source account,
      ensuring that money is never created or destroyed, only moved.

* **Financial Analysis & Forecasting**:
    * **Historical View**: Get a cumulative, day-by-day balance for any account up to the present.
    * **Future Forecast**: Project account balances into the future based on all scheduled recurring transactions and
      income.
    * **Per-Account Statistics**: Analyze account performance with metrics like starting/ending balances for a period
      and lowest/highest balances.

* **Advanced Categorization & Reporting**:
    * **Hierarchical Tagging**: Apply tags to both transactions and accounts. Tags can be nested (e.g.,
      `Expenses:Food:Groceries`) to allow for detailed, tree-based reporting on spending and income.
    * **Cross-Account Insights**: Use tags to see total expenses or income in a category, regardless of which account
      was used.

* **Interoperability**:
    * **Ledger CLI Compatibility**: Every entity (accounts, transactions, tags) can be configured with a `ledger_name`,
      allowing the entire financial history to be exported in a format compatible with the powerful, plain-text
      accounting tool, [Ledger](https://www.ledger-cli.org/).

## Project Goals

The main goals of this project are:

1. **Current Account State Visibility**: Provide a clear and accurate view of all account states over time
2. **Account Forecasting**: Enable precise forecasting of account balances based on recurring transactions and income

## Technical Stack

This project is built using the following technologies:

* **Backend**:
    * [Rust](https://www.rust-lang.org/) - Programming language (Edition 2024)
    * [Axum](https://github.com/tokio-rs/axum) - Web framework for building REST APIs
    * [SeaORM](https://www.sea-ql.org/SeaORM/) - Async ORM for Rust
    * [Polars](https://pola.rs/) - Data manipulation and analysis library
    * [Tokio](https://tokio.rs/) - Async runtime
    * [Tower](https://github.com/tower-rs/tower) - Middleware and service abstractions

* **API Documentation**:
    * [utoipa](https://github.com/juhaku/utoipa) - OpenAPI 3.0 specification generation
    * [utoipa-swagger-ui](https://github.com/juhaku/utoipa) - Interactive Swagger UI

* **Monitoring & Observability**:
    * [tracing](https://github.com/tokio-rs/tracing) - Structured logging and diagnostics
    * [axum-prometheus](https://github.com/Ptrskay3/axum-prometheus) - Prometheus metrics integration

* **Data Handling**:
    * [serde](https://serde.rs/) - Serialization/deserialization
    * [chrono](https://github.com/chronotope/chrono) - Date and time handling
    * [rust_decimal](https://github.com/paupino/rust-decimal) - Precise decimal arithmetic
    * [validator](https://github.com/Keats/validator) - Input validation

* **Database**:
    * SQLite (development)
    * PostgreSQL (production)

* **CLI & Configuration**:
    * [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
    * [config](https://github.com/mehcode/config-rs) - Configuration management
    * [dotenvy](https://github.com/allan2/dotenvy) - Environment variable loading

* **Frontend**:
    * Separate application (not included in this repository)

## Getting Started

### Prerequisites

* [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
* [SQLite](https://www.sqlite.org/download.html) (for development)
* [PostgreSQL](https://www.postgresql.org/download/) (for production)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/finrust.git
   cd finrust
   ```

2. Build the project:
   ```bash
   cargo build
   ```

### Running the Application

1. Initialize the database:
   ```bash
   cargo run init-db --database-url "sqlite://finrust.db"
   ```

2. Start the server:
   ```bash
   cargo run serve
   ```

   Or with custom configuration:
   ```bash
   cargo run serve --database-url "sqlite://finrust.db" --bind-address "0.0.0.0:3000"
   ```

3. Access the application:
   - **API Base URL**: `http://localhost:3000/api/v1/`
   - **Swagger UI Documentation**: `http://localhost:3000/swagger-ui`
   - **Health Check**: `http://localhost:3000/health`
   - **Prometheus Metrics**: `http://localhost:3000/metrics`

## API Documentation

The application provides a comprehensive REST API with full OpenAPI 3.0 specification and interactive Swagger UI documentation.

### Available Endpoints

- **Accounts**: CRUD operations for financial accounts
- **Transactions**: Manage one-off, recurring, and imported transactions
- **Recurring Income**: Handle recurring income streams
- **Users**: User management functionality
- **Statistics**: Account performance metrics and analytics
- **Timeseries**: Historical and forecasted account balance data
- **Manual Account States**: Override account balances at specific points in time

### Interactive Documentation

Once the server is running, visit `http://localhost:3000/swagger-ui` to explore the complete API documentation with:
- Interactive endpoint testing
- Request/response schemas
- Authentication requirements
- Example payloads

## Project Structure

```
finrust/
├── Cargo.toml              # Main workspace configuration
├── src/                    # Main application code
│   ├── handlers/           # API endpoint handlers
│   ├── cli.rs             # Command-line interface
│   ├── router.rs          # API routing configuration
│   └── main.rs            # Application entry point
└── workspace/
    ├── model/              # Database models and entities
    │   └── src/
    │       └── entities/   # Entity definitions
    ├── migration/          # Database migrations
    │   └── src/            # Migration scripts
    ├── compute/            # Financial computation logic
    │   └── src/
    └── common/             # Shared utilities and types
        └── src/
```

## Development

### Setting Up Development Environment

1. Install Rust and required dependencies:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup update stable
   ```

2. Set up the database:
   ```bash
   # For SQLite (development)
   # No additional setup required

   # For PostgreSQL (production)
   # Create a database and update the connection string in your configuration
   ```

### Code Standards

#### Logging

This project uses the `tracing` crate for structured logging. Follow these guidelines for logging:

- Use appropriate log levels:
  - `error!` - For errors that prevent the application from functioning correctly
  - `warn!` - For unexpected conditions that don't prevent the application from working
  - `info!` - For important events that should be visible in normal operation
  - `debug!` - For detailed information useful during development
  - `trace!` - For very detailed diagnostic information

- Never use `println!` in production code; always use the appropriate tracing macro
- Use the `#[instrument]` attribute on functions to automatically trace function entry and exit

#### Documentation

All code should be well-documented following these guidelines:

- Every public struct, enum, trait, and function must have a docstring
- Docstrings should explain the purpose, behavior, and usage of the item
- Use the standard Rust documentation format with sections like `# Arguments`, `# Returns`, etc.
- Follow the DRY principle: don't explain what the code is doing line by line, but focus on the intent and special cases

### Running Tests

Run the test suite with:

```bash
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
