/// To run: cargo run --example run-analysis
use std::{collections::HashMap, str::FromStr, sync::Arc};

use anyhow::Result;
use ethers::types::{H160, U256};
use ethrpc::{http::HttpTransport, Web3, Web3Transport};
use reqwest::Client;
use tycho_core::{
    models::{blockchain::BlockTag, token::TokenOwnerStore},
    traits::TokenAnalyzer,
    Bytes,
};
use tycho_ethereum::{token_analyzer::trace_call::TraceCallDetector, BytesConvertible};
use url::Url;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let rpc = std::env::var("RPC_URL").expect("RPC URL must be set for testing");
    let transport = Web3Transport::new(HttpTransport::new(
        Client::new(),
        Url::from_str(&rpc).unwrap(),
        "transport".to_owned(),
    ));
    let w3 = Web3::new(transport);
    let tf = TokenOwnerStore::new(HashMap::from([(
        Bytes::from_str("3A9FfF453d50D4Ac52A6890647b823379ba36B9E").unwrap(),
        (
            Bytes::from_str("260E069deAd76baAC587B5141bB606Ef8b9Bab6c").unwrap(),
            U256::from_dec_str("13042252617814040589")
                .unwrap()
                .to_bytes(),
        ),
    )]));

    let trace_call = TraceCallDetector {
        web3: w3,
        finder: Arc::new(tf),
        settlement_contract: H160::from_str("0xc9f2e6ea1637E499406986ac50ddC92401ce1f58").unwrap(),
    };

    let quality = trace_call
        .analyze(
            H160::from_str("0x3A9FfF453d50D4Ac52A6890647b823379ba36B9E")
                .unwrap()
                .to_bytes(),
            BlockTag::Latest,
        )
        .await
        .unwrap();

    println!("{:?}", quality);
    Ok(())
}
