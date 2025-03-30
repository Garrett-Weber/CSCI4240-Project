# My Cargo Workspace

This project is a Cargo workspace that contains a binary crate and a library crate.

## Crates

### Binary Crate

- **Name**: binary-crate
- **Description**: This crate serves as the executable for the workspace.
- **Entry Point**: The main function is located in `binary-crate/src/main.rs`.

### Library Crate

- **Name**: library-crate
- **Description**: This crate provides shared functionality that can be used by the binary crate and other consumers.
- **Public API**: The library's API is defined in `library-crate/src/lib.rs`.

## Getting Started

To build and run the binary crate, navigate to the workspace directory and use the following command:

```
cargo run --bin binary-crate
```

To build the library crate, use:

```
cargo build --package library-crate
```

## License

This project is licensed under the MIT License. See the LICENSE file for more details.