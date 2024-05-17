use std::error::Error;
use solana_sdk::{pubkey, pubkey::Pubkey};
use tokio::task;

mod lib_gen;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let kapital: u64 = 100000000; // 0,1 SOL
    let bps: u16 = 1000;
    pub const WALLET: Pubkey = pubkey!("3AugJd4PLZPgXFdhj4gb52U57FGUr3FjifHSuLejRaxM");
    let private_key_str: &str = "3Detg1HDHh4RgKmTJnZMUubatoxgWGm37wqPJ6kJLpbnQ9yfh1q7MHjRHCwvHMT7bcD6ZqT24P8ve9UGb4xmLpzH";
    let addresses: Vec<&str> = ["J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn", "8wXtPeU6557ETkp9WHFY1n1EcU6NxDvbAggHGsMYiHsB"].to_vec();
    

    for address in addresses {
        loop {

            let task: task::JoinHandle<bool> = task::spawn(lib_gen::start(kapital, bps, private_key_str, WALLET, address));

            let _result: bool = task.await.unwrap();

        }
    }

    Ok(())
}
