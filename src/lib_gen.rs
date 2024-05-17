use reqwest;
use bincode;
use bs58;

use serde_json::json;
use tokio::time::{sleep, Duration};

use std::collections::HashMap;

use jupiter_swap_api_client::{
    quote::QuoteRequest, swap::SwapRequest, transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{pubkey, transaction::VersionedTransaction};
use std::str::FromStr;


pub async fn start(kapital: u64, bps: u16, private_key_str: &str, wallet:Pubkey, address: &str) -> bool {
    let jupiter_swap_api_client: JupiterSwapApiClient = JupiterSwapApiClient::new(format!("https://quote-api.jup.ag/v6"));

    let input_mint_: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
    let output_mint_: Pubkey = solana_sdk::pubkey::Pubkey::from_str(address).unwrap();

    let amount: u64 = kapital - 105000;

    let quote_resp: jupiter_swap_api_client::quote::QuoteResponse = quote_response(jupiter_swap_api_client.clone(), amount, input_mint_, output_mint_, bps).await;

    let versioned_transaction_in: VersionedTransaction = swap_response(jupiter_swap_api_client.clone(), wallet, quote_resp.clone()).await;

    let b: bool = sign(versioned_transaction_in, private_key_str).await;

    if b == false {
        return false; // Не смог купить токены, процесс сначала начинается
    }



    let (token_amount, decimals): (f64, u64) = get_amount_to_account(address).await;

    let amount: u64 = (token_amount * decimals as f64) as u64;

    let result: bool = generator(address).await;

    if result == false {
        println!("А всё, токен {} всё . _.", address);
        return false; // Прошло 48 часов и цена не поднялась
    }

    let quote_resp: jupiter_swap_api_client::quote::QuoteResponse = quote_response(jupiter_swap_api_client.clone(), amount, input_mint_, output_mint_, bps).await;

    let versioned_transaction_out: VersionedTransaction = swap_response(jupiter_swap_api_client.clone(), wallet, quote_resp.clone()).await;

    let b: bool = sign(versioned_transaction_out, private_key_str).await;

    if b == false {
        return false; // Не смог продать купленные ранее токены, тут запись в db 
    }

    println!("Победа, получается?");

    return true
}


pub async fn quote_response(jupiter_swap_api_client: JupiterSwapApiClient, amount: u64, input_mint_: Pubkey, output_mint_: Pubkey, bps: u16) -> jupiter_swap_api_client::quote::QuoteResponse {
    let quote_request = QuoteRequest {
        amount: amount,
        input_mint: input_mint_,
        output_mint: output_mint_,
        slippage_bps: bps,
        ..QuoteRequest::default()
    };

    let quote_response: jupiter_swap_api_client::quote::QuoteResponse = jupiter_swap_api_client.quote(&quote_request).await.unwrap();

    quote_response
}


pub async fn swap_response(jupiter_swap_api_client: JupiterSwapApiClient, wallet: Pubkey, quote_response: jupiter_swap_api_client::quote::QuoteResponse) -> VersionedTransaction {
    let swap_response_in = jupiter_swap_api_client
        .swap(&SwapRequest {
            user_public_key: wallet,
            quote_response: quote_response.clone(),
            config: TransactionConfig::default(),
        })
        .await
        .unwrap();

    let versioned_transaction_in: VersionedTransaction = bincode::deserialize(&swap_response_in.swap_transaction).unwrap();

    versioned_transaction_in
}

pub async fn get_amount_to_account(address: &str) -> (f64, u64) {

    // let rpc = "http://144.202.63.224:3013/proxy";
    let rpc = "http://localhost:8899";
    let client = reqwest::Client::new();

    let mut headers = reqwest::header::HeaderMap::new();
    // headers.insert("X-API-KEY", "aa-aa-00-aa".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let params = vec![
        json!("3AugJd4PLZPgXFdhj4gb52U57FGUr3FjifHSuLejRaxM"),
        json!({
            "mint": address
        }),
        json!({
            "encoding": "jsonParsed"
        })
    ];

    let json_data = json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "getTokenAccountsByOwner",
        "params": params
    });

    let res = client.post(rpc)
        .headers(headers)
        .json(&json_data)
        .send()
        .await.unwrap();

    let response: HashMap<String, serde_json::Value> = res.json().await.unwrap();
    let tok_am: f64 = *&response["result"]["value"][0]["account"]["data"]["parsed"]["info"]["tokenAmount"]["uiAmount"].as_f64().unwrap();
    let tok_dec: u64 = *&response["result"]["value"][0]["account"]["data"]["parsed"]["info"]["tokenAmount"]["decimals"].as_u64().unwrap();

    (tok_am, tok_dec)

}

async fn get_amount(address: &str) -> f64 {
    let url = format!("https://api.dexscreener.io/latest/dex/tokens/{}", address);
    let client = reqwest::Client::new();

    let res = client.post(url)
        .send()
        .await.unwrap();

    let response: HashMap<String, serde_json::Value> = res.json().await.unwrap();
    let amount: f64 = *&response["pairs"][0]["priceUsd"].as_str().unwrap().parse().unwrap();

    amount
}

pub async fn generator(address: &str) -> bool {
    let sell_old: f64 = get_amount(address).await;

    let dur: u64 = 3;

    for _index in 0..(dur*60*60) {

        let sell_new: f64 = get_amount(address).await;

        if sell_new / sell_old >= 1.3 {
            return true;
        } else {
            sleep(Duration::from_secs(1)).await;
        }
    }

    let dur: u64 = 21;

    let mut percent: f64 = 1.3;
    let percent_next: f64 = 1.2;

    let pr: f64 = (percent - percent_next) / dur as f64;

    for _index in 0..(dur*60*60) {

        let sell_new: f64 = get_amount(address).await;

        if sell_new / sell_old >= percent {
            return true;
        } else {
            sleep(Duration::from_secs(1)).await;
        }

        percent -= pr;
    }

    let dur: u64 = 12;

    let mut percent: f64 = 1.2;
    let percent_next: f64 = 1.15;

    let pr: f64 = (percent - percent_next) / dur as f64;

    for _index in 0..(dur*60*60) {

        let sell_new: f64 = get_amount(address).await;

        if sell_new / sell_old >= percent {
            return true;
        } else {
            sleep(Duration::from_secs(1)).await;
        }

        percent -= pr;
    }

    let dur: u64 = 6;

    let mut percent: f64 = 1.15;
    let percent_next: f64 = 1.1;

    let pr: f64 = (percent - percent_next) / dur as f64;

    for _index in 0..(dur*60*60) {

        let sell_new: f64 = get_amount(address).await;

        if sell_new / sell_old >= percent {
            return true;
        } else {
            sleep(Duration::from_secs(1)).await;
        }

        percent -= pr;
    }

    let dur: u64 = 6;

    let mut percent: f64 = 1.1;
    let percent_next: f64 = 1.03;

    let pr: f64 = (percent - percent_next) / dur as f64;

    for _index in 0..(dur*60*60) {

        let sell_new: f64 = get_amount(address).await;

        if sell_new / sell_old >= percent {
            return true;
        } else {
            sleep(Duration::from_secs(1)).await;
        }

        percent -= pr;
    }

    return false;
}

pub async fn sign(mut versioned_transaction: VersionedTransaction, private_key_str: &str) -> bool {
    let rpc_client: RpcClient = RpcClient::new("http://localhost:8899".into());

    let private_key_bytes = bs58::decode(private_key_str).into_vec().unwrap();
    let keypair = match Keypair::from_bytes(&private_key_bytes) {
        Ok(keypair) => keypair,
        Err(_) => panic!("Неизвестная ошибка"),
    };

    let latest_blockhash = rpc_client.get_latest_blockhash().await.unwrap();

    versioned_transaction
        .message
        .set_recent_blockhash(latest_blockhash);

    let signed_versioned_transaction = VersionedTransaction::try_new(versioned_transaction.message, &[&keypair]).unwrap();

    let flag: bool;

    let result = rpc_client.send_and_confirm_transaction(&signed_versioned_transaction).await;
    match result {
        Ok(signature) => {
            println!("Tx | {}", signature);
            flag = true;
        },
        Err(err) => {
            eprintln!("Error | {:?}", err);
            flag = false;
        }
    }

    flag
}