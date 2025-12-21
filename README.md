# **FinRust \- Home Finance Tracker**

This repository contains the full-stack solution for a powerful, self-hosted home finance tracking application. Built
with **Rust**, **Axum**, **SeaORM**, and **Polars** on the backend, and **Yew (WebAssembly)** on the frontend, this tool
is designed for users who want granular control over their financial data, robust forecasting capabilities, and a system
based on sound accounting principles.

The core mission of this project is to provide a comprehensive and accurate view of your financial situation, both past
and future. It allows you to model your entire financial ecosystem—from various bank accounts and currencies to complex
recurring transactions—and then use that model to gain insights and forecast with precision.

## **Table of Contents**

* [Core Features](https://www.google.com/search?q=%23core-features&authuser=1)
* [Project Goals](https://www.google.com/search?q=%23project-goals&authuser=1)
* [Technical Stack](https://www.google.com/search?q=%23technical-stack&authuser=1)
* [Getting Started](https://www.google.com/search?q=%23getting-started&authuser=1)
* [API Documentation](https://www.google.com/search?q=%23api-documentation&authuser=1)
* [Project Structure](https://www.google.com/search?q=%23project-structure&authuser=1)
* [Development](https://www.google.com/search?q=%23development&authuser=1)
* [Contributing](https://www.google.com/search?q=%23contributing&authuser=1)
* [License](https://www.google.com/search?q=%23license&authuser=1)

## **Core Features**

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
    * **Scenarios (What-If Analysis)**: Create and apply hypothetical financial scenarios (e.g., "Buying a House", "New
      Job") to see how major decisions impact your future balance without affecting your actual data.
    * **Per-Account Statistics**: Analyze account performance with metrics like starting/ending balances for a period
      and lowest/highest balances.
* **Advanced Categorization & Reporting**:
    * **Hierarchical Tagging**: Apply tags to both transactions and accounts. Tags can be nested (e.g.,
      `Expenses:Food:Groceries`) to allow for detailed, tree-based reporting on spending and income.
    * **Cross-Account Insights**: Use tags to see total expenses or income in a category, regardless of which account
      was used.
* **Manual Account States**:
    * Create "snapshots" of your account balance at specific points in time.
    * Useful for correcting drift or ensuring historical accuracy by overriding calculated balances with actual bank
      statement values.
* **Interoperability**:
    * **Ledger CLI Compatibility**: Every entity (accounts, transactions, tags) can be configured with a `ledger_name`,
      allowing the entire financial history to be exported in a format compatible with the powerful, plain-text
      accounting tool, [Ledger](https://www.ledger-cli.org/).

## **Project Goals**

The main goals of this project are:

1. **Current Account State Visibility**: Provide a clear and accurate view of all account states over time.
2. **Account Forecasting**: Enable precise forecasting of account balances based on recurring transactions and income.
3. **Scenario Planning**: Allow users to model future life events and their financial impact.

## **Technical Stack**

This project is built using the following technologies:

**Backend**

* [Rust](https://www.rust-lang.org/) (Edition 2024\)
* [Axum](https://github.com/tokio-rs/axum) \- REST API
* [SeaORM](https://www.sea-ql.org/SeaORM/) \- Async ORM
* [Polars](https://pola.rs/) \- High-performance dataframes for financial compute
* [Tokio](https://tokio.rs/) \- Async runtime
* **Observability**: `tracing` & `axum-prometheus`
* **Documentation**: `utoipa` (OpenAPI 3.0)

**Frontend**

* [Yew](https://yew.rs/) \- Rust framework for WebAssembly apps
* [Trunk](https://trunkrs.dev/) \- WASM web application bundler
* **Styling**: [Tailwind CSS](https://tailwindcss.com/) & [DaisyUI](https://daisyui.com/)
* **Routing**: Yew Router

**Infrastructure & Data**

* **Database**: SQLite (development), PostgreSQL (production)
* **Containerization**: Docker & Docker Compose
* **Data Handling**: `serde`, `chrono`, `rust_decimal`

## **Getting Started**

### **Using Docker Compose (Recommended)**

The easiest way to run the full stack (backend \+ frontend) is via Docker Compose.

1. **Clone the repository**:  
   Bash

```
git clone https://github.com/yourusername/finrust.git
cd finrust
```

2.
3. **Start the application**:  
   Bash

```
docker-compose up -d
```

4.
5. **Access the services**:
    * **Frontend**: `http://localhost:8081`
    * **Backend API**: `http://localhost:8080/api/v1/`
    * **Swagger UI**: `http://localhost:8080/swagger-ui`
6. *Note: Data will be persisted in the `./data` directory.*

### **Manual Installation**

**Prerequisites**

* [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
* [Trunk](https://trunkrs.dev/) (for frontend): `cargo install trunk`
* `wasm-bindgen-cli`: `cargo install wasm-bindgen-cli`
* [SQLite](https://www.sqlite.org/download.html) (for development) or PostgreSQL

**Backend Setup**

1. Initialize the database:  
   Bash

```
cargo run init-db --database-url "sqlite://finrust.db"
```

2.
3. Start the server:  
   Bash

```
cargo run serve --bind-address "0.0.0.0:8080"
```

4.

**Frontend Setup**

1. Navigate to the frontend workspace:  
   Bash

```
cd workspace/frontend
```

2.
3. Add the WebAssembly target (if not already added):  
   Bash

```
rustup target add wasm32-unknown-unknown
```

4.
5. Serve the application:  
   Bash

```
trunk serve --port 8081
```

6.

## **API Documentation**

The application provides a comprehensive REST API with full OpenAPI 3.0 specification and interactive Swagger UI
documentation.

### **Available Endpoints**

* **Accounts**: CRUD operations for financial accounts.
* **Transactions**: Manage one-off, recurring, and imported transactions.
* **Recurring Income**: Handle recurring income streams.
* **Scenarios**: Create and manage "what-if" financial scenarios.
* **Manual Account States**: Override account balances at specific points in time.
* **Statistics**: Account performance metrics and analytics.
* **Timeseries**: Historical and forecasted account balance data.
* **Users**: User management functionality.
* **Tags & Categories**: Hierarchical organization for transactions.

### **Interactive Documentation**

Once the backend is running, visit `http://localhost:8080/swagger-ui` to explore the complete API documentation with
interactive endpoint testing, schemas, and example payloads.

## **Project Structure**

Plaintext

```
finrust/
├── Cargo.toml              # Main workspace configuration
├── docker-compose.yaml     # Docker composition for full stack
├── Dockerfile              # Multi-stage Docker build
├── src/                    # Main application entry point & CLI
│   ├── handlers/           # API endpoint handlers
│   ├── cli.rs              # Command-line interface definitions
│   └── router.rs           # API routing configuration
└── workspace/
    ├── frontend/           # Yew (WebAssembly) Frontend Application
    │   ├── src/
    │   └── index.html
    ├── model/              # Database models and entities (SeaORM)
    ├── migration/          # Database migrations
    ├── compute/            # Financial computation logic (Polars)
    └── common/             # Shared utilities and types
```

## **Development**

### **Setting Up Development Environment**

1. **Install Rust and dependencies**:  
   Bash

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
cargo install trunk wasm-bindgen-cli
```

2.
3. **Set up the database**:  
   Bash

```
# For SQLite (development)
cargo run init-db --database-url "sqlite://finrust.db"
```

4.

### **Code Standards**

* **Logging**: This project uses the `tracing` crate. Use appropriate log levels (`error!`, `warn!`, `info!`, `debug!`,
  `trace!`) and never `println!` in production code.
* **Documentation**: Every public struct, enum, trait, and function must have a docstring explaining its purpose,
  arguments, and return values.

### **Running Tests**

Run the test suite for the entire workspace:

Bash

```
cargo test
```

## **Contributing**

Contributions are welcome\! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## **License**

This project is licensed under the MIT License \- see the [LICENSE](https://www.google.com/search?q=LICENSE&authuser=1)
file for details.
