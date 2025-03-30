/// A library for solana utilities, providing useful functions for blockchain exploration/dev

pub mod tools {
    use base64::prelude::*;
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use solana_account_decoder::UiAccountEncoding;
    use solana_cli_output::display::println_transaction;
    use solana_client::{
        rpc_client::RpcClient,
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
    };
    use solana_sdk::{account::Account, pubkey::Pubkey};
    use solana_sdk::{
        message::Message,
        transaction::{Transaction, VersionedTransaction},
    };
    use std::collections::HashMap;
    use std::error::Error;
    use std::str::FromStr as _;

    const DISCRIMINATOR_LEN: usize = 8;

    // Get program accounts by discriminator
    pub fn get_program_accounts_with_discrim(
        connection: &RpcClient,
        program_address: &str,
        discrim: &[u8],
    ) -> Result<Vec<(Pubkey, Account)>, Box<dyn Error>> {
        let discrim_base64 = BASE64_STANDARD.encode(discrim);
        let memcmp = RpcFilterType::Memcmp(Memcmp::new(0, MemcmpEncodedBytes::Base64(discrim_base64)));
        
        let config = create_program_account_config(vec![memcmp]);
        
        let accounts = connection
            .get_program_accounts_with_config(&Pubkey::from_str(program_address)?, config)
            .map_err(|e| format!("RPC error: {:?}", e))?;
            
        Ok(accounts)
    }

    // Helper function to create program account config with filters
    fn create_program_account_config(filters: Vec<RpcFilterType>) -> RpcProgramAccountsConfig {
        RpcProgramAccountsConfig {
            filters: Some(filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    // Decode message data
    pub fn decode_message(message_data: &str) {
        let decoded_data = BASE64_STANDARD
            .decode(message_data)
            .expect("Failed to decode message");
        let message: Message = bincode::deserialize(&decoded_data).unwrap();
        let tx = Transaction::new_unsigned(message);
        println_transaction(&VersionedTransaction::from(tx), None, " ", None, None);
    }

    // Find accounts by variable value
    pub fn find_accounts_by_variable(
        connection: &RpcClient,
        program_address: &str,
        discrim: &[u8],
        variable_offset: usize,
        variable_value: &[u8],
    ) -> Result<Vec<(Pubkey, Account)>, Box<dyn Error>> {
        let program_pubkey = Pubkey::from_str(program_address)?;

        let discrim_filter = RpcFilterType::Memcmp(Memcmp::new(0, MemcmpEncodedBytes::Bytes(discrim.into())));
        let variable_filter = RpcFilterType::Memcmp(Memcmp::new(
            variable_offset,
            MemcmpEncodedBytes::Bytes(variable_value.into()),
        ));

        let config = create_program_account_config(vec![discrim_filter, variable_filter]);

        let accounts = connection.get_program_accounts_with_config(&program_pubkey, config)?;
        Ok(accounts)
    }

    // Calculate discriminator for an account
    pub fn calculate_discriminator(account_name: &str) -> [u8; DISCRIMINATOR_LEN] {
        let mut hasher = Sha256::new();
        hasher.update(format!("account:{}", account_name).as_bytes());
        let hash = hasher.finalize();
        let mut discriminator = [0u8; DISCRIMINATOR_LEN];
        discriminator.copy_from_slice(&hash[..DISCRIMINATOR_LEN]);
        discriminator
    }

    // Find accounts by criteria (comprehensive search)
    pub fn find_accounts_by_criteria(
        connection: &RpcClient,
        idl: &str,
        program_address: &str,
        account_name: &str,
        variable_path: &str,
        variable_value: &[u8],
    ) -> Result<Vec<(Pubkey, Account)>, Box<dyn Error>> {
        // Calculate the discriminator for the account name
        let discriminator = calculate_discriminator(account_name);

        // Get the offset of the variable
        let variable_offset = get_variable_offset_from_idl(idl, account_name, variable_path)?;

        // Encode the discriminator and variable value in Base64
        let discriminator_base64 = BASE64_STANDARD.encode(discriminator);
        let variable_value_base64 = BASE64_STANDARD.encode(variable_value);

        // Fetch accounts matching the discriminator and variable value
        let program_pubkey = Pubkey::from_str(&program_address)?;

        let discrim_filter = RpcFilterType::Memcmp(
            Memcmp::new(0, MemcmpEncodedBytes::Base64(discriminator_base64))
        );
        let variable_filter = RpcFilterType::Memcmp(
            Memcmp::new(variable_offset, MemcmpEncodedBytes::Base64(variable_value_base64))
        );

        let config = create_program_account_config(vec![discrim_filter, variable_filter]);
        let accounts = connection.get_program_accounts_with_config(&program_pubkey, config)?;
        
        Ok(accounts)
    }

    // IDL parsing and variable resolving functions
    pub fn get_variable_offset_from_idl(
        idl: &str,
        account_name: &str,
        variable_path: &str,
    ) -> Result<usize, Box<dyn Error>> {
        // Parse the IDL JSON
        let idl_json: Value = serde_json::from_str(idl)?;
        let types_map = build_types_map(&idl_json)?;
        
        // Find the account definition in the IDL
        let account = find_account_in_idl(&idl_json, account_name)?;

        // Split the variable path into components (e.g., "pricing.tradeImpactFeeScalar")
        let path_parts: Vec<&str> = variable_path.split('.').collect();

        // Calculate the offset of the variable
        let mut offset = 0;
        let mut current_fields = extract_account_fields(account)?;

        for part in path_parts.clone() {
            let mut field_found = false;

            for field in current_fields {
                let field_name = field
                    .get("name")
                    .and_then(|n| n.as_str())
                    .ok_or("Field has no name")?;

                let field_type = field.get("type").ok_or("Field has no type")?;

                if field_name == part {
                    field_found = true;

                    if part == *path_parts.last().unwrap() {
                        // If this is the last part of the path, return the offset
                        return Ok(offset + DISCRIMINATOR_LEN);
                    }

                    // Resolve the type and continue traversing
                    current_fields = resolve_nested_fields(field_type, &types_map)?;
                    break;
                }

                // Accumulate the size of the current field
                offset += calculate_field_size(field_type, &types_map)?;
            }

            if !field_found {
                return Err(format!("Field '{}' not found", part).into());
            }
        }

        Err("Variable not found in account fields".into())
    }

    // Helper function to build types map from IDL
    fn build_types_map<'a>(idl_json: &'a Value) -> Result<HashMap<String, &'a Value>, Box<dyn Error>> {
        let types = idl_json
            .get("types")
            .and_then(|t| t.as_array())
            .ok_or("IDL does not contain 'types' or it is not an array")?;

        Ok(types
            .iter()
            .filter_map(|t| {
                let name = t.get("name")?.as_str()?;
                Some((name.to_string(), t))
            })
            .collect())
    }

    // Helper function to find account in IDL
    fn find_account_in_idl<'a>(idl_json: &'a Value, account_name: &str) -> Result<&'a Value, Box<dyn Error>> {
        let accounts = idl_json
            .get("accounts")
            .ok_or("IDL does not contain 'accounts' field")?
            .as_array()
            .ok_or("'accounts' field is not an array")?;

        accounts
            .iter()
            .find(|acc| acc.get("name").map_or(false, |name| name == account_name))
            .ok_or_else(|| "Account not found in IDL".into())
    }

    // Helper function to extract account fields
    fn extract_account_fields(account: &Value) -> Result<&Vec<Value>, Box<dyn Error>> {
        account
            .get("type")
            .and_then(|t| t.get("fields"))
            .ok_or("Account type does not contain 'fields'")?
            .as_array()
            .ok_or_else(|| "Fields is not an array".into())
    }

    // Helper to resolve nested fields
    fn resolve_nested_fields<'a>(
        field_type: &Value,
        types_map: &'a HashMap<String, &Value>,
    ) -> Result<&'a Vec<Value>, Box<dyn Error>> {
        if let Some(defined_type) = field_type.get("defined").and_then(|t| t.as_str()) {
            let custom_type_def = types_map
                .get(defined_type)
                .ok_or_else(|| format!("Unknown defined type: {}", defined_type))?;
            let fields = custom_type_def
                .get("type")
                .and_then(|t| t.get("fields"))
                .ok_or("Custom type does not contain 'fields'")?
                .as_array()
                .ok_or("'fields' is not an array")?;
            Ok(fields)
        } else {
            Err("Field type is not a nested struct".into())
        }
    }

    // Calculate field sizes for offset determination
    fn calculate_field_size(
        field_type: &Value,
        types_map: &HashMap<String, &Value>,
    ) -> Result<usize, Box<dyn Error>> {
        match field_type {
            Value::String(type_str) => match type_str.as_str() {
                "u8" | "i8" => Ok(1),
                "u16" | "i16" => Ok(2),
                "u32" | "i32" | "f32" => Ok(4),
                "u64" | "i64" | "f64" => Ok(8),
                "u128" | "i128" => Ok(16),
                "bool" => Ok(1),
                "publicKey" => Ok(32),
                "string" => Err("Dynamic size types like 'string' are not supported".into()),
                custom_type => {
                    let custom_type_def = types_map
                        .get(custom_type)
                        .ok_or_else(|| format!("Unknown custom type: {}", custom_type))?;
                    calculate_custom_type_size(custom_type_def, types_map)
                }
            },
            Value::Object(obj) if obj.get("array").is_some() => {
                let array = obj.get("array").ok_or("Array type is invalid")?;
                let array_type = array.get(0).ok_or("Array type is missing")?;
                let array_length = array
                    .get(1)
                    .and_then(|len| len.as_u64())
                    .ok_or("Array length is invalid")?;
                Ok(calculate_field_size(array_type, types_map)? * array_length as usize)
            }
            Value::Object(obj) if obj.get("defined").is_some() => {
                let defined_type = obj
                    .get("defined")
                    .and_then(|t| t.as_str())
                    .ok_or("Invalid 'defined' type")?;
                let custom_type_def = types_map
                    .get(defined_type)
                    .ok_or_else(|| format!("Unknown defined type: {}", defined_type))?;
                calculate_custom_type_size(custom_type_def, types_map)
            }
            Value::Object(obj) if obj.get("option").is_some() => {
                let option_type = obj.get("option").ok_or("Option type is invalid")?;
                calculate_field_size(option_type, types_map)
            }
            Value::Object(obj) if obj.get("coption").is_some() => {
                let coption_type = obj.get("coption").ok_or("COption type is invalid")?;
                calculate_field_size(coption_type, types_map)
            }
            Value::Object(obj) if obj.get("tuple").is_some() => {
                let tuple = obj.get("tuple").ok_or("Tuple type is invalid")?;
                let tuple_elements = tuple
                    .as_array()
                    .ok_or("Tuple elements must be an array")?;
                let mut size = 0;
                for element in tuple_elements {
                    size += calculate_field_size(element, types_map)?;
                }
                Ok(size)
            }
            _ => Err(format!("Unsupported field type: {:?}", field_type).into()),
        }
    }

    fn calculate_custom_type_size(
        custom_type_def: &Value,
        types_map: &HashMap<String, &Value>,
    ) -> Result<usize, Box<dyn Error>> {
        let type_kind = custom_type_def
            .get("type")
            .ok_or("Custom type does not contain 'type'")?;

        match type_kind.get("kind").and_then(|k| k.as_str()) {
            Some("struct") => {
                let fields = type_kind
                    .get("fields")
                    .ok_or("Struct type does not contain 'fields'")?
                    .as_array()
                    .ok_or("'fields' is not an array")?;

                let mut size = 0;
                for field in fields {
                    let field_type = field.get("type").ok_or("Field has no type")?;
                    size += calculate_field_size(field_type, types_map)?;
                }
                Ok(size)
            }
            Some("enum") => {
                let variants = type_kind
                    .get("variants")
                    .ok_or("Enum type does not contain 'variants'")?
                    .as_array()
                    .ok_or("'variants' is not an array")?;

                // Enums are typically represented as a discriminant (u8) plus the largest variant size
                let mut max_variant_size = 0;
                for variant in variants {
                    let empty_vec = Vec::new();
                    let variant_fields = variant
                            .get("fields")
                            .and_then(|f| f.as_array())
                            .unwrap_or(&empty_vec);

                    let mut variant_size = 0;
                    for field in variant_fields {
                        variant_size += calculate_field_size(field, types_map)?;
                    }
                    max_variant_size = max_variant_size.max(variant_size);
                }
                Ok(1 + max_variant_size) // 1 byte for the discriminant
            }
            _ => Err("Unsupported custom type kind".into()),
        }
    }

    /// Decodes a byte array into a value based on the specified type.
    pub fn decode_value_by_type(bytes: &[u8], offset: usize, type_str: &str) -> Result<String, Box<dyn Error>> {
        match type_str {
            "u8" => bytes
                .get(offset)
                .map(|&v| v.to_string())
                .ok_or_else(|| "Failed to extract u8 value".into()),
            "u64" => {
                let value_bytes = bytes
                    .get(offset..offset + 8)
                    .ok_or_else(|| -> Box<dyn std::error::Error> { "Failed to extract u64 value".into() })?;
                Ok(u64::from_le_bytes(value_bytes.try_into().unwrap()).to_string())
            },
            "i64" => {
                let value_bytes = bytes
                    .get(offset..offset + 8)
                    .ok_or_else(|| -> Box<dyn std::error::Error> { "Failed to extract i64 value".into() })?;
                Ok(i64::from_le_bytes(value_bytes.try_into().unwrap()).to_string())
            },
            "f64" => {
                let value_bytes = bytes
                    .get(offset..offset + 8)
                    .ok_or_else(|| -> Box<dyn std::error::Error> { "Failed to extract f64 value".into() })?;
                Ok(f64::from_le_bytes(value_bytes.try_into().unwrap()).to_string())
            },
            "bool" => bytes
                .get(offset)
                .map(|&v| (v != 0).to_string())
                .ok_or_else(|| "Failed to extract bool value".into()),
            "publicKey" => {
                let value_bytes = bytes
                    .get(offset..offset + 32)
                    .ok_or_else(|| -> Box<dyn std::error::Error> { "Failed to extract publicKey value".into() })?;
                Ok(Pubkey::new_from_array(value_bytes.try_into().expect("Incorrectly sized pubkey")).to_string())
            },
            _ => Err(format!("Unsupported variable type: {}", type_str).into()),
        }
    }

    /// Encodes a value string into bytes based on the specified type.
    pub fn encode_value_by_type(value_str: &str, type_str: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        match type_str {
            "u8" => Ok(vec![value_str.parse::<u8>().map_err(|_| "Failed to parse u8")?]),
            "u64" => Ok(value_str
                .parse::<u64>()
                .map_err(|_| "Failed to parse u64")?
                .to_le_bytes()
                .to_vec()),
            "i64" => Ok(value_str
                .parse::<i64>()
                .map_err(|_| "Failed to parse i64")?
                .to_le_bytes()
                .to_vec()),
            "f64" => Ok(value_str
                .parse::<f64>()
                .map_err(|_| "Failed to parse f64")?
                .to_le_bytes()
                .to_vec()),
            "bool" => Ok(vec![value_str.parse::<bool>().map_err(|_| "Failed to parse bool")? as u8]),
            "publicKey" => Ok(Pubkey::from_str(value_str)
                .map_err(|_| "Failed to parse publicKey")?
                .to_bytes()
                .to_vec()),
            _ => Err(format!("Unsupported variable type: {}", type_str).into()),
        }
    }

    /// Extracts the value of a variable from account data based on its offset and type in the IDL.
    pub fn extract_variable_value(
        data: &[u8],
        idl: &str,
        account_name: &str,
        variable_path: &str,
    ) -> Result<String, Box<dyn Error>> {
        // Get the offset of the variable
        let offset = get_variable_offset_from_idl(idl, account_name, variable_path)?;

        // Determine the type of the variable
        let variable_type = get_variable_type_from_idl(idl, account_name, variable_path)?;

        // Decode the value based on its type
        decode_value_by_type(data, offset, &variable_type)
    }

    /// Extracts the type of a variable from the IDL.
    pub fn get_variable_type_from_idl(
        idl: &str,
        account_name: &str,
        variable_path: &str,
    ) -> Result<String, Box<dyn Error>> {
        let idl_json: Value = serde_json::from_str(idl)?;
        let account = find_account_in_idl(&idl_json, account_name)?;
        let mut current_fields = extract_account_fields(account)?;

        for part in variable_path.split('.') {
            let field = current_fields
                .iter()
                .find(|f| f.get("name").map_or(false, |name| name == part))
                .ok_or(format!("Field '{}' not found", part))?;

            if part == variable_path.split('.').last().unwrap() {
                return field
                    .get("type")
                    .and_then(|t| t.as_str())
                    .map(String::from)
                    .ok_or("Field type not found".into());
            }

            current_fields = field
                .get("type")
                .and_then(|t| t.get("fields"))
                .ok_or("Field type does not contain 'fields'")?
                .as_array()
                .ok_or("'fields' is not an array")?;
        }

        Err("Variable type not found".into())
    }

    // Support function (placeholder)
    pub fn deploy_program_with_fireblocks() {
        use solana_cli::program;
        program::process_program_subcommand(todo!(), todo!(), todo!());
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::tools::{calculate_discriminator, get_program_accounts_with_discrim};

    use super::tools;
    use base64::prelude::*;
    use solana_client::rpc_client::RpcClient;
    use solana_sdk::{
        message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer,
        transaction::Transaction,
    };

    #[test]
    fn test_decode_transaction() {
        let payer = Keypair::new();
        let ix =
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 10);
        let message = Message::new(&[ix], Some(&(payer.pubkey())));
        let tx: Transaction = Transaction::new_unsigned(message);
        let message_data = BASE64_STANDARD.encode(tx.message_data());
        println!("Message: {}", message_data);
        super::tools::decode_message(&message_data);
    }

    #[test]
    fn test_find_accounts_by_variable() {
        let rpc_client = RpcClient::new("https://mainnet.helius-rpc.com/?api-key=".to_string());
        let program_address = Pubkey::new_unique().to_string();
        let discriminator = [0; 8]; // Example discriminator
        let variable_offset = 8; // Example offset for the variable in account data
        let variable_value = Pubkey::new_unique().to_bytes(); // Example variable value (e.g., token.owner)

        let accounts = tools::find_accounts_by_variable(
            &rpc_client,
            &program_address,
            &discriminator,
            variable_offset,
            &variable_value,
        )
        .expect("Failed to find accounts");

        println!("Found accounts: {:?}", accounts);
        assert!(accounts.is_empty()); // Example assertion
    }

    #[test]
    fn test_get_variable_offset_from_idl() {

        let lol = fs::read_to_string("./test/perpetuals.json").unwrap();


        let offset = tools::get_variable_offset_from_idl(&lol, "Custody", "pool")
            .expect("Failed to get variable offset");
        assert_eq!(offset, 97);
    }

    #[test]
    fn test_get_program_accounts_with_discrim() {
        let rpc_client = RpcClient::new("https://mainnet.helius-rpc.com/?api-key=".to_string());
        let accounts = get_program_accounts_with_discrim(&rpc_client, "PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu", &calculate_discriminator("Custody")).unwrap();

        println!("{}", accounts.len());
        
    }

    #[test]
    fn test_find_accounts_by_criteria() {
        let rpc_client = RpcClient::new("https://mainnet.helius-rpc.com/?api-key=".to_string());
        let idl = fs::read_to_string("./test/perpetuals.json").unwrap();

        let account_name = "Custody";
        let variable_path = "pricing.tradeImpactFeeScalar";
        //let variable_value = Pubkey::from_str_const("5BUwFW4nRbftYTDMbgxykoFWqWHPzahFSNAaaaJtVKsq").to_bytes(); // Example value
        let variable_value = 1250000000000000u64.to_le_bytes(); // Example value

        let accounts = tools::find_accounts_by_criteria(
            &rpc_client,
            &idl,
            "PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu",
            account_name,
            variable_path,
            &variable_value,
        )
        .expect("Failed to find accounts");

        println!("Number of accounts found: {}", accounts.len());
        assert!(accounts.len() > 0); // Ensure the function runs without errors
    }

    #[test]
    fn test_calculate_discriminator() {
        let account_name = "Custody";
        let discriminator = calculate_discriminator(account_name);
        println!("Discriminator for '{}': {:?}", account_name, discriminator);
        assert_eq!(discriminator.len(), 8);
    }
}
