# FinRust Frontend

A modern web frontend for the FinRust financial management application, built with Yew and WebAssembly.

## Technology Stack

- **Yew**: Modern Rust framework for creating multi-threaded frontend web apps with WebAssembly
- **WebAssembly**: High-performance web applications compiled from Rust
- **Tailwind CSS**: Utility-first CSS framework for rapid UI development
- **Daisy UI**: Component library built on top of Tailwind CSS
- **Yew Router**: Client-side routing for single-page applications

## Features

- Responsive design with mobile-first approach
- Modern component-based architecture
- Type-safe frontend development with Rust
- Integration with FinRust backend APIs
- Theme switching (light/dark mode)
- Fast performance with WebAssembly

## Development

### Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- `trunk` for building and serving the application
- `wasm-bindgen-cli` for WebAssembly bindings

### Setup

1. Install the required tools:
```bash
# Install trunk
cargo install trunk

# Install wasm-bindgen-cli
cargo install wasm-bindgen-cli

# Add WebAssembly target
rustup target add wasm32-unknown-unknown
```

2. Build and serve the application:
```bash
cd workspace/frontend
trunk serve
```

3. Open your browser and navigate to `http://localhost:8080`

### Building for Production

```bash
cd workspace/frontend
trunk build --release
```

The built files will be available in the `dist/` directory.

## Project Structure

```
src/
├── lib.rs          # Main application entry point
├── components/     # Reusable UI components
│   ├── mod.rs
│   └── navbar.rs   # Navigation bar component
└── pages/          # Page components
    ├── mod.rs
    ├── home.rs     # Home page
    └── about.rs    # About page
```

## Styling

The application uses Tailwind CSS with Daisy UI components for consistent and modern styling. The CSS framework is loaded via CDN in the `index.html` file.

### Available Themes

- Light theme (default)
- Dark theme
- Additional Daisy UI themes can be configured

## API Integration

The frontend communicates with the FinRust backend through RESTful APIs. HTTP requests are handled using `gloo-net` and `reqwest` with WebAssembly support.