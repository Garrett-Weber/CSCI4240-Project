use base64::prelude::*;
use clap::Parser;
use sol_tools::tools::{
    calculate_discriminator, extract_variable_value, find_accounts_by_criteria, get_variable_type_from_idl,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;

/// CLI for searching Solana accounts by account name, variable path, and value.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Custom RPC URL
    #[arg(short, long = "rpc", value_name = "RPC_URL")]
    rpc: String,

    /// Path to the IDL JSON file
    #[arg(short, long = "idl", value_name = "IDL_PATH")]
    idl: String,

    /// Program ID of the Solana program
    #[arg(short, long = "program", value_name = "PROGRAM_ID")]
    program: String,

    /// Name of the account to search
    #[arg(short = 'n', long = "name", value_name = "ACCOUNT_NAME")]
    account: String,

    /// Path to the variable in the account
    #[arg(short, long = "path", value_name = "VARIABLE_PATH")]
    variable: String,

    /// Value of the variable to search for
    #[arg(short = 'k', long, value_name = "VARIABLE_VALUE")]
    value: String,

    /// File to output results if there are too many accounts
    #[arg(short, long = "output", value_name = "OUTPUT_FILE")]
    output: Option<String>,

    /// Variable of interest to analyze
    #[arg(short = 's', long, value_name = "INTEREST_VARIABLE")]
    interest: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    // Load the IDL
    let idl = std::fs::read_to_string(&cli.idl).expect("Failed to read IDL file");

    // Search for accounts
    let accounts = search_accounts(&cli, &idl);
    
    // Handle results
    handle_results(&accounts, &cli.output);

    // Analyze variable of interest if provided
    if let Some(interest) = &cli.interest {
        analyze_variable_of_interest(&accounts, &idl, &cli.account, interest);
    }
}

// Function to parse value based on variable type
fn search_accounts(cli: &Cli, idl: &str) -> Vec<(Pubkey, solana_sdk::account::Account)> {
    // Determine the type of the variable from the IDL
    let variable_type = get_variable_type_from_idl(idl, &cli.account, &cli.variable)
        .expect("Failed to determine variable type from IDL");

    // Parse the variable value based on its type
    let parsed_value = parse_value_by_type(&variable_type, &cli.value);

    // Create an RPC client
    let rpc_client = RpcClient::new(cli.rpc.clone());

    // Search for accounts
    find_accounts_by_criteria(
        &rpc_client,
        idl,
        &cli.program,
        &cli.account,
        &cli.variable,
        &parsed_value,
    )
    .expect("Failed to find accounts")
}

// Parse value based on type
fn parse_value_by_type(variable_type: &str, value_str: &str) -> Vec<u8> {
    sol_tools::tools::encode_value_by_type(value_str, variable_type)
        .unwrap_or_else(|e| panic!("Failed to encode value: {}", e))
}

// Handle search results
fn handle_results(accounts: &[(Pubkey, solana_sdk::account::Account)], output_file: &Option<String>) {
    if accounts.len() <= 5 {
        display_accounts(accounts, accounts.len());
    } else {
        println!("Found {} accounts. Displaying the first 4:", accounts.len());
        display_accounts(accounts, 4);

        if let Some(output_path) = output_file {
            save_accounts_to_file(accounts, output_path);
            println!("Results written to {}", output_path);
        } else {
            println!("Too many accounts. Use --output to save results to a file.");
        }
    }
}

// Display accounts
fn display_accounts(accounts: &[(Pubkey, solana_sdk::account::Account)], limit: usize) {
    println!("Found {} accounts:", accounts.len());
    for (i, (pubkey, account)) in accounts.iter().take(limit).enumerate() {
        println!("{}. Pubkey: {}", i + 1, pubkey);
        println!("   Data: {}", BASE64_STANDARD.encode(&account.data));
    }
}

// Save accounts to file
fn save_accounts_to_file(accounts: &[(Pubkey, solana_sdk::account::Account)], path: &str) {
    let mut file = File::create(path).expect("Failed to create output file");
    for (pubkey, account) in accounts {
        writeln!(file, "Pubkey: {}", pubkey).unwrap();
        writeln!(file, "Data: {:?}", account.data).unwrap();
    }
}

// Analyze variable of interest
fn analyze_variable_of_interest(
    accounts: &[(Pubkey, solana_sdk::account::Account)], 
    idl: &str,
    account_name: &str, 
    interest: &str
) {
    let mut values = Vec::new();
    for (_, account) in accounts {
        if let Ok(value) = extract_variable_value(&account.data, idl, account_name, interest) {
            values.push(value);
        }
    }

    // Count occurrences
    let mut counts: HashMap<String, usize> = HashMap::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }

    // Sort by count in descending order
    let mut sorted_counts: Vec<_> = counts.into_iter().collect();
    sorted_counts.sort_by(|a, b| b.1.cmp(&a.1));

    // Display results
    println!("Top 5 most common values for '{}':", interest);
    for (value, count) in sorted_counts.into_iter().take(5) {
        println!("Value: {}, Count: {}", value, count);
    }
}