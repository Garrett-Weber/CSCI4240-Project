# Solana Blockchain Tools

A collection of command-line utilities and libraries for exploring and interacting with Solana accounts.

## Overview

This project consists of two main components:

1. **sol-tools**: A Rust library that provides core functionality for Solana blockchain account parsing, inspection, and analysis.
2. **sol-util**: A command-line interface that uses sol-tools to provide a user-friendly way to search and analyze accounts.

## Features

- **Account Search**: Find accounts by name, variable path, and values
- **IDL-Based Account Analysis**: Parse accounts and extract values based on IDL definitions
- **Variable Value Analysis**: Analyze variable values across multiple accounts
- **Discriminator Calculation**: Calculate and use account discriminators for efficient searching
- **Minimal RPC Usage**: Optimized queries to minimize RPC calls

## Installation

### Prerequisites

- Rust and Cargo (latest stable version)
- Solana CLI tools

### Building from Source

```bash
git clone https://github.com/yourusername/sol-tools.git
cd sol-tools
cargo build --release
```

The binaries will be available in the `target/release` directory.

## Usage

### CLI Tool

Search for all accounts of a specific type:

```bash
sol-util \
  --rpc https://api.mainnet-beta.solana.com \
  --idl ./path/to/idl.json \
  --program PROGRAM_ID \
  --name ACCOUNT_NAME
```

Search for accounts with specific criteria:

```bash
sol-util \
  --rpc https://api.mainnet-beta.solana.com \
  --idl ./path/to/idl.json \
  --program PROGRAM_ID \
  --name ACCOUNT_NAME \
  --path variable.path \
  --value VALUE
```

Search for accounts with multiple criteria (all must match):

```bash
sol-util \
  --rpc https://api.mainnet-beta.solana.com \
  --idl ./path/to/idl.json \
  --program PROGRAM_ID \
  --name ACCOUNT_NAME \
  --path variable.path1 --value VALUE1 \
  --path variable.path2 --value VALUE2
```

#### Examples

```bash
# Find Custody accounts with maxLeverage of "BUvduFTd2sWFagCunBPLupG8fBTJqweLw9DuhruNFSCm" and print most common isStable values
sol-util --  -r "https://mainnet.helius-rpc.com/?api-key=" -i ./sol-tools/test/perpetuals.json -p PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu -n Custody --path tokenAccount --value "BUvduFTd2sWFagCunBPLupG8fBTJqweLw9DuhruNFSCm" -s isStable

# Find PositionRequest accounts with specific Custody and find most common sizeUsdDelta values
sol-util --  -r "https://mainnet.helius-rpc.com/?api-key=" -i ./sol-tools/test/perpetuals.json -p PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu -n PositionRequest --path custody --value "7xS2gz2bTp3fwCC7knJvUWTEU9Tycczu6VhJYKgi1wdz"  --interest sizeUsdDelta
```

### Library Usage

You can use the `sol-tools` library in your Rust projects:

```rust
use sol_tools::tools;
use solana_client::rpc_client::RpcClient;

fn main() {
    let connection = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let idl = std::fs::read_to_string("./idl/program.json").unwrap();
    
    // Find accounts by criteria
    let accounts = tools::find_accounts_by_criteria(
        &connection,
        &idl,
        "PROGRAM_ID",
        "AccountName",
        "variable.path",
        &variable_value_bytes,
    ).unwrap();
    
    println!("Found {} accounts", accounts.len());
}
```

## Available Commands

### Account Search

| Flag | Description | Example |
|------|-------------|---------|
| `--rpc` | RPC endpoint URL | `--rpc https://api.mainnet-beta.solana.com` |
| `--idl` | Path to IDL JSON file | `--idl ./idl/program.json` |
| `--program` | Program ID | `--program PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu` |
| `--name` | Account name | `--name Custody` |
| `--path` | Variable path (can be used multiple times) | `--path pricing.maxLeverage --path isStable` |
| `--value` | Variable value (must match the number of paths) | `--value 5000000 --value true` |
| `--output` | Output file path (optional) | `--output results.txt` |
| `--interest` | Variable to analyze (optional) | `--interest pricing.buffer` |
| `--limit` | Maximum number of accounts to display | `--limit 10` |

## Advanced Usage

### Variable Types

The tool handles various Solana account field types:

- `u8`, `i8`
- `u16`, `i16`
- `u32`, `i32`, `f32`
- `u64`, `i64`, `f64`
- `u128`, `i128`
- `bool`
- `publicKey` (Solana addresses)

### Account Discriminators

Solana accounts often start with an 8-byte discriminator that identifies the account type. This library calculates these discriminators using:

```
SHA256("account:" + account_name)[0..8]
```

## Developing

### Project Structure

```
project/
├── sol-tools/      # Core library functionality
│   ├── src/
│   │   └── lib.rs  # Library implementation
│   └── Cargo.toml
├── sol-util/       # CLI application
│   ├── src/
│   │   └── main.rs # CLI interface
│   └── Cargo.toml
└── Cargo.toml      # Workspace definition
```

## License

MIT
