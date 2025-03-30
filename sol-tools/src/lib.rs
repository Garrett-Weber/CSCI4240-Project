/// A library for solana utilities, providing useful functions for blockchain exploration/dev

pub mod tools {
    use std::str::FromStr as _;
    use solana_cli_output::display::println_transaction;
    use solana_sdk::{
        message::Message, pubkey::Pubkey, transaction::{Transaction, VersionedTransaction}
    };

    use solana_client::{
        rpc_client::RpcClient, rpc_config::RpcAccountInfoConfig,
    };
    use base64::prelude::*;

    
    
    fn get_program_accounts_with_discrim(
        connection: &RpcClient,
        program_address: &str,
        discrim: &[u8],
    ) -> Result<
        Vec<(solana_sdk::pubkey::Pubkey, solana_sdk::account::Account)>,
        Box<dyn std::error::Error>,
    > {
        use solana_client::{
            rpc_config::RpcProgramAccountsConfig,
            rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
        };
        use solana_account_decoder::UiAccountEncoding;

        let memcmp =
            RpcFilterType::Memcmp(Memcmp::new(0, MemcmpEncodedBytes::Bytes(discrim.into())));
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![memcmp]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            ..Default::default()
        };
        let accounts = connection.get_program_accounts_with_config(
            &solana_sdk::pubkey::Pubkey::from_str(program_address)?,
            config,
        )?;
        Ok(accounts)
    }

    pub fn deploy_program_with_fireblocks() {
        use solana_cli::program;

        program::process_program_subcommand(todo!(), todo!(), todo!());
    }

    pub fn decode_message(message_data: &str) {
        use solana_sdk::message::Message;

        // Decode the base64 message data
        let decoded_data = BASE64_STANDARD.decode(message_data).expect("Failed to decode message");
        let message: Message = bincode::deserialize(&decoded_data).unwrap();
        let tx = Transaction::new_unsigned(message);
        println_transaction(&VersionedTransaction::from(tx), None, " ", None, None);
    }

}


#[cfg(test)]
mod tests {
    use base64::prelude::*;
    use solana_sdk::{message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};


    #[test]
    fn test_decode_transaction() {
        let payer = Keypair::new();
        let ix = solana_sdk::system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 10);
        let message = Message::new(&[ix], Some(&(payer.pubkey())));
        let tx: Transaction = Transaction::new_unsigned(message);
        let message_data = BASE64_STANDARD.encode(tx.message_data());
        println!("Message: {}", message_data);
        super::tools::decode_message(&message_data);
    }
}
