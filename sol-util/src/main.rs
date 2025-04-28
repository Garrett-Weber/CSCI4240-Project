use base64::prelude::*;
use clap::Parser;
use sol_tools::tools::{
    calculate_discriminator, extract_variable_value, find_accounts_by_criteria, get_program_accounts_with_discrim,
    get_variable_type_from_idl, encode_value_by_type,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

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

    /// Path to the variable in the account (can be specified multiple times)
    #[arg(short, long = "path", value_name = "VARIABLE_PATH")]
    variable_paths: Vec<String>,

    /// Value of the variable to search for (order must match paths)
    #[arg(short = 'k', long = "value", value_name = "VARIABLE_VALUE")]
    values: Vec<String>,

    /// File to output results if there are too many accounts
    #[arg(short, long = "output", value_name = "OUTPUT_FILE")]
    output: Option<String>,

    /// Variable of interest to analyze
    #[arg(short = 's', long, value_name = "INTEREST_VARIABLE")]
    interest: Option<String>,
    
    /// Maximum number of accounts to display in the console
    #[arg(long = "limit", value_name = "DISPLAY_LIMIT", default_value = "5")]
    display_limit: usize,
}

/// A constraint with path and value for filtering accounts
struct PathValueConstraint {
    path: String,
    value: Vec<u8>,
    offset: usize,
}

fn main() {
    let cli = Cli::parse();

    // Load the IDL
    let idl = std::fs::read_to_string(&cli.idl).expect("Failed to read IDL file");

    // Validate the number of paths and values
    if !cli.variable_paths.is_empty() && cli.variable_paths.len() != cli.values.len() {
        eprintln!("Error: The number of paths and values must match");
        std::process::exit(1);
    }

    // Search for accounts
    let accounts = if cli.variable_paths.is_empty() {
        // Just search by account discriminator
        search_accounts_by_account_name(&cli)
    } else {
        // Search by multiple path-value pairs
        search_accounts_with_multiple_criteria(&cli, &idl)
    };
    
    // Handle results
    handle_results(&accounts, &cli.output, cli.display_limit);

    // Analyze variable of interest if provided
    if let Some(interest) = &cli.interest {
        analyze_variable_of_interest(&accounts, &idl, &cli.account, interest);
    }
}

// Search accounts by discriminator only
fn search_accounts_by_account_name(cli: &Cli) -> Vec<(Pubkey, Account)> {
    // Create an RPC client
    let rpc_client = RpcClient::new(cli.rpc.clone());
    
    // Calculate discriminator for the account name
    let discriminator = calculate_discriminator(&cli.account);
    
    // Search for accounts with just the discriminator
    println!("Searching for all {} accounts...", cli.account);
    get_program_accounts_with_discrim(
        &rpc_client,
        &cli.program,
        &discriminator,
    )
    .unwrap_or_else(|e| {
        eprintln!("Error fetching accounts: {}", e);
        Vec::new()
    })
}

// Parse constraints from CLI arguments
fn parse_constraints(cli: &Cli, idl: &str) -> Vec<PathValueConstraint> {
    let mut constraints = Vec::new();
    
    for (i, path) in cli.variable_paths.iter().enumerate() {
        let value_str = &cli.values[i];
        
        // Get variable type from IDL
        let variable_type = get_variable_type_from_idl(idl, &cli.account, path)
            .unwrap_or_else(|e| {
                eprintln!("Error getting type for path {}: {}", path, e);
                std::process::exit(1);
            });
        
        // Encode the value according to the type
        let encoded_value = encode_value_by_type(value_str, &variable_type)
            .unwrap_or_else(|e| {
                eprintln!("Error encoding value for path {}: {}", path, e);
                std::process::exit(1);
            });
        
        // Get the offset for this variable
        let offset = sol_tools::tools::get_variable_offset_from_idl(idl, &cli.account, path)
            .unwrap_or_else(|e| {
                eprintln!("Error getting offset for path {}: {}", path, e);
                std::process::exit(1);
            });
        
        constraints.push(PathValueConstraint {
            path: path.clone(),
            value: encoded_value,
            offset,
        });
    }
    
    constraints
}

// Search accounts with multiple constraints
fn search_accounts_with_multiple_criteria(cli: &Cli, idl: &str) -> Vec<(Pubkey, Account)> {
    // Create an RPC client
    let rpc_client = RpcClient::new(cli.rpc.clone());
    
    // Parse all constraints
    let constraints = parse_constraints(cli, idl);
    
    if constraints.is_empty() {
        return search_accounts_by_account_name(cli);
    }
    
    // Get the first constraint to start the search
    let first = &constraints[0];
    println!("Searching for {} accounts with {} constraints...", cli.account, constraints.len());
    
    // Initial search with the first constraint
    let mut accounts = find_accounts_by_criteria(
        &rpc_client,
        idl,
        &cli.program,
        &cli.account,
        &first.path,
        &first.value,
    )
    .unwrap_or_else(|e| {
        eprintln!("Error searching accounts with initial constraint: {}", e);
        Vec::new()
    });
    
    if accounts.is_empty() || constraints.len() == 1 {
        return accounts;
    }
    
    // Apply remaining constraints by filtering the accounts
    for constraint in constraints.iter().skip(1) {
        println!("Applying additional constraint: path={}", constraint.path);
        accounts = filter_accounts_by_constraint(&accounts, constraint);
    }
    
    accounts
}

// Filter accounts by a specific constraint
fn filter_accounts_by_constraint(accounts: &[(Pubkey, Account)], constraint: &PathValueConstraint) -> Vec<(Pubkey, Account)> {
    let mut filtered = Vec::new();
    
    for (pubkey, account) in accounts {
        // Check if the account meets this constraint
        if account.data.len() >= constraint.offset + constraint.value.len() {
            let slice = &account.data[constraint.offset..constraint.offset + constraint.value.len()];
            if slice == constraint.value.as_slice() {
                filtered.push((pubkey.clone(), account.clone()));
            }
        }
    }
    
    filtered
}

// Display accounts
fn display_accounts(accounts: &[(Pubkey, Account)], limit: usize) {
    println!("Found {} accounts:", accounts.len());
    for (i, (pubkey, account)) in accounts.iter().take(limit).enumerate() {
        println!("{}. Pubkey: {}", i + 1, pubkey);
        println!("   Data Length: {} bytes", account.data.len());
        println!("   Lamports: {}", account.lamports);
    }
}

// Handle search results
fn handle_results(accounts: &[(Pubkey, Account)], output_file: &Option<String>, display_limit: usize) {
    if accounts.is_empty() {
        println!("No accounts found matching the criteria.");
        return;
    }
    
    if accounts.len() <= display_limit {
        display_accounts(accounts, accounts.len());
    } else {
        display_accounts(accounts, display_limit);
        println!("\nShowing {} of {} accounts found.", display_limit, accounts.len());
        
        if let Some(output_path) = output_file {
            save_accounts_to_file(accounts, output_path);
            println!("Full results written to {}", output_path);
        } else {
            println!("To see all accounts, use --output to save results to a file.");
        }
    }
}

// Save accounts to file in JSON format
fn save_accounts_to_file(accounts: &[(Pubkey, Account)], path: &str) {
    let mut file = File::create(path).expect("Failed to create output file");
    
    // Create a JSON structure for all accounts
    let mut json_accounts = serde_json::json!({
        "count": accounts.len(),
        "accounts": []
    });
    
    // Add each account to the accounts array
    let accounts_array = json_accounts["accounts"].as_array_mut().unwrap();
    
    for (pubkey, account) in accounts {
        // Extract any interesting variables if available and the IDL is loaded
        let variables = serde_json::Map::new();
        
        // Add the account data
        let account_json = serde_json::json!({
            "pubkey": pubkey.to_string(),
            "data": BASE64_STANDARD.encode(&account.data),
            "data_length": account.data.len(),
            "lamports": account.lamports,
            "owner": account.owner.to_string(),
            "executable": account.executable,
            "rent_epoch": account.rent_epoch,
            "extracted_variables": variables
        });
        
        accounts_array.push(account_json);
    }
    
    // Write pretty-printed JSON to file
    let formatted_json = serde_json::to_string_pretty(&json_accounts)
        .expect("Failed to format JSON");
    
    write!(file, "{}", formatted_json).expect("Failed to write to output file");
    
    println!("Full results written to {} in JSON format", path);
}

// Analyze variable of interest
fn analyze_variable_of_interest(
    accounts: &[(Pubkey, Account)], 
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