# Quickstart

How to swap on-chain with Tycho. This quickstart will help you:

* Fetch real-time market data from Tycho Indexer.
* Simulate swaps between token pairs. This lets you calculate **spot prices** and **output amounts** using Tycho Simulation.
* Encode the best trade for given token pairs.
* Simulate or execute the best trade using Tycho Execution.

{% hint style="success" %}
Want to chat with our docs? Download an LLM-friendly [text file of the full Tycho docs](https://files.gitbook.com/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FjrIe0oInIEt65tHqWn2w%2Fuploads%2FMYcEejawelGMbi4DTLSR%2Ftycho_docs_monofile.txt?alt=media\&token=64b8f6ee-0f2e-4684-a5d2-7f3b859196e4).
{% endhint %}

## Run the Quickstart

Clone the [Tycho Simulation repository](https://github.com/propeller-heads/tycho-simulation); here's [the quickstart](https://github.com/propeller-heads/tycho-simulation/blob/main/examples/quickstart/main.rs) code.

Run the quickstart with execution using the following commands:

{% tabs %}
{% tab title="Mainnet" %}
```sh
export RPC_URL=https://ethereum.publicnode.com
export PK=<your-private-key>
cargo run --release --example quickstart -- --swapper-pk $PK
```
{% endtab %}

{% tab title="Base" %}
```bash
export RPC_URL=https://base-rpc.publicnode.com
export PK=<your-private-key>
cargo run --release --example quickstart -- --chain base --swapper-pk $PK
```
{% endtab %}

{% tab title="Unichain" %}
```sh
export RPC_URL=https://unichain-rpc.publicnode.com
export PK=<your-private-key>
cargo run --release --example quickstart -- --chain unichain --swapper-pk $PK
```
{% endtab %}
{% endtabs %}

If you don't have an RPC URL, here are some public ones for [Ethereum Mainnet](https://ethereumnodes.com/), [Unichain](https://chainlist.org/chain/130), and [Base](https://chainlist.org/chain/8453).

The `--swapper-pk` flag is unnecessary if you want to run the quickstart without simulation or execution.

### What it does

The quickstart fetches all protocol states. Then it returns the best amount out (best price) for a given token pair (by default, 10 USDC to WETH).

Additionally, it returns calldata to execute the swap on this pool with the Tycho Router.

You should see an output like this:

```
Looking for pool with best price for 10 USDC -> WETH

==================== Received block 14222319 ====================

The best swap (out of 6 possible pools) is:
Protocol: "uniswap_v3"
Pool address: "0x65081cb48d74a32e9ccfed75164b8c09972dbcf1"
Swap: 10.000000 USDC -> 0.006293 WETH 
Price: 0.000629 WETH per USDC, 1589.052587 USDC per WETH

Signer private key was not provided. Skipping simulation/execution...
```

If you want to see results for a different token, amount, or chain, you can set additional flags:

```bash
export TYCHO_URL=<tycho-api-url-for-chain>
export TYCHO_API_KEY=<tycho-api-key-for-chain>
cargo run --release --example quickstart -- --sell-token "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913" --buy-token "0x4200000000000000000000000000000000000006" --sell-amount 10 --chain "base" --swapper-pk $PK
```

This example would seek the best swap for 10 USDC -> WETH on Base.

#### Logs

If you want to see all the Tycho Indexer and Simulation logs, run with `RUST_LOG=info`:

```bash
RUST_LOG=info cargo run --release --example quickstart --swapper-pk $PK
```

## How the quickstart works

The quickstart shows you how to:

1. **Set up and load** necessary data, like available tokens.
2. **Connect to the Tycho Indexer** to fetch on-chain protocol data (e.g., Uniswap V2, Balancer V2) and **build a Protocol Stream** that streams updates, like new pools and states, in real-time.
3. **Simulate** swaps on all available pools for a specified pair (e.g., USDC, WETH), and print out the most USDC available for 1 WETH.
4. **Encode** a swap to swap 1 WETH against the best pool.
5. **Execute** the swap against the Tycho Router.

### 1. Set up

Run Tycho Indexer by setting up the following environment variables:

* URL (by default `"tycho-beta.propellerheads.xyz"`)
* API key (by default, the test key is `sampletoken`)
* TVL threshold: This is a filter for snapshot data for pools with TVL greater than the specified threshold (in ETH). Its default is **10,000 ETH** to limit the data you pull.

The Indexer stream or the Simulation does not manage tokens; you manage them yourself.

To simplify this, [load\_all\_tokens](for-solvers/simulation.md#step-1-fetch-tokens) gets all current token information from Tycho Indexer RPC for you.

### 2. Connect to Tycho Indexer

The protocol stream connects to Tycho Indexer to fetch the real-time state of protocols.

```rust
let mut protocol_stream = ProtocolStreamBuilder::new(&tycho_url, Chain::Ethereum)
    .exchange::<UniswapV2State>("uniswap_v2", tvl_filter.clone(), None)
    .exchange::<EVMPoolState<PreCachedDB>>(
        "vm:balancer_v2",
        tvl_filter.clone(),
        Some(balancer_pool_filter),
    )
    .auth_key(Some(tycho_api_key.clone()))
    .set_tokens(all_tokens.clone())
    .await
    .build()
    .await
    .expect("Failed building protocol stream");
```

Here, you only subscribe to Uniswap V2 and Balancer V2. To include additional protocols like Uniswap V3, simply add:

```rust
.exchange::<UniswapV3State>("uniswap_v3", tvl_filter.clone(), None)
```

For a full list of supported protocols and which simulation state (like `UniswapV3State`) they use, see [Supported Protocols](for-solvers/supported-protocols.md).

Note: The protocol stream supplies **all** protocol states in the first `BlockUpdate` object. All subsequent `BlockUpdates` contain only new and changed protocol states (i.e., deltas).

### 3. Simulate swap

`get_best_swap` uses Tycho Simulation to simulate swaps and calculate buy amounts. We inspect all **protocols** updated in the current block (i.e., protocols with balance changes).

#### a. Simulating token swaps

```rust
let result = state.get_amount_out(amount_in, &tokens[0], &tokens[1])
```

`result` is a `GetAmountOutResult` containing information on amount out, gas cost, and the protocol's new state. So you could follow your current swap with another.

```rust
let other_result = result.new_state.get_amount_out(other_amount_in, &tokens[0], &tokens[1])
```

By inspecting each of the amount outs, you can then choose the protocol component with the highest amount out.

### 4. Encode a swap

After choosing the best swap, you can use Tycho Execution to encode it.

**a. Create encoder**

```rust
let encoder = TychoRouterEncoderBuilder::new()
    .chain(chain)
    .user_transfer_type(UserTransferType::TransferFromPermit2)
    .build()
    .expect("Failed to build encoder");
```

#### b. Create a solution object

Now you know the best protocol component (i.e., pool), you can compute a minimum amount out. And you can put the swap into the expected input format for your encoder.&#x20;

The minimum amount out is a very important parameter to set in Tycho Execution. The value acts as a guardrail and protects your funds during execution against MEV. This quickstart accepts a slippage of 0.25% over the simulated amount out.&#x20;

```rust
let slippage = 0.0025; // 0.25% slippage
let bps = BigUint::from(10_000u32);
let slippage_percent = BigUint::from((slippage * 10000.0) as u32);
let multiplier = &bps - slippage_percent;
let min_amount_out = (expected_amount * &multiplier) / &bps;
```

:warning:  For maximum security, you should determine the minimum amount should from a **third-party source.**

After this, you can create the Swap and Solution objects. For more info about the `Swap` and `Solution` models, see [here](for-solvers/execution/encoding.md#solution-struct).

```rust
let simple_swap = Swap::new(
    protocol_component,
    sell_token.address.clone(),
    buy_token.address.clone(),
    // Split defines the fraction of the amount to be swapped.
    0f64,
);

// Then we create a solution object with the previous swap
let solution = Solution {
    sender: user_address.clone(),
    receiver: user_address,
    given_token: sell_token.address,
    given_amount: sell_amount,
    checked_token: buy_token.address,
    exact_out: false,     // it's an exact in solution
    checked_amount: min_amount_out,
    swaps: vec![simple_swap],
    ..Default::default()
};
```

#### c. Encode swap

```rust
let encoded_solution = encoder
    .encode_solutions(vec![solution.clone()])
    .expect("Failed to encode router calldata")[0]
```

### 5. Encode full method calldata

You need to build the full calldata for the router. Tycho handles the swap encoding, but you control the full input to the router method. This quickstart provides helper functions (`encode_tycho_router_call`  and `sign_permit`)

Use it as follows:

```rust
let tx = encode_tycho_router_call(
    named_chain.into(),
    encoded_solution.clone(),
    &solution,
    chain.native_token().address,
    signer.clone(),
)
.expect("Failed to encode router call");
```

:warning: These functions are only examples intended for use within the quickstart.\
**Do not use them in production.** You must write your own logic to:

* Control parameters like `minAmountOut`, `receiver`, and transfer type.
* Sign the permit2 object safely and correctly.

This gives you full control over execution. And it protects you from MEV and slippage risks.

### 6. Simulate or execute the best swap

This step allows you to test or perform real transactions based on the best available swap options. For this step, you need to pass your wallet's private key in the run command. Handle it securely and never expose it publicly.

```bash
cargo run --release --example quickstart -- --swapper-pk $PK
```

When you provide your private key, the quickstart will check your token balances and display them before showing you options:

```
Your balance: 100.000000 USDC
Your WETH balance: 1.500000 WETH
```

If you don't have enough tokens for the swap, you'll see a warning:

```
Your balance: 5.000000 USDC
⚠️ Warning: Insufficient balance for swap. You have 5.000000 USDC but need 10.000000 USDC
Your WETH balance: 1.500000 WETH
```

You'll then encounter the following prompt:

```
Would you like to simulate or execute this swap?
Please be aware that the market might move while you make your decision, which might lead to a revert if you've set a min amount out or slippage.
Warning: slippage is set to 0.25% during execution by default.

? What would you like to do? ›
❯ Simulate the swap
  Execute the swap
  Skip this swap
```

You have three options:

1. **Simulate the swap:** Tests the swap without executing it on-chain. It simulates an approval (for permit2) and a swap transaction on the node. You'll see something like this:

```
Simulating by performing an approval (for permit2) and a swap transaction...

Simulated Block 21944458:
  Transaction 1: Status: true, Gas Used: 46098
  Transaction 2: Status: true, Gas Used: 182743
```

If status is `false`, the simulation has failed. You can print the full simulation output for detailed failure information.

2. **Execute the swap:** Performs the swap on-chain using your real funds. The process performs an approval (for permit2) and a swap transaction. You'll receive transaction hashes and statuses like this:

```
Executing by performing an approval (for permit2) and a swap transaction...

Approval transaction sent with hash: 0xf2a9217016397b09f5274e225754029ebda31743b4da7dd1441e13971e1f43b0 and status: true

Swap transaction sent with hash: 0x0b26c9965b4ee39b5646ab93070f018c027ac3d0c9d56548a6db4412be7abbc8 and status: true

✅ Swap executed successfully! Exiting the session...

Summary: Swapped 10.000000 USDC → 0.006293 WETH at a price of 0.000629 WETH per USDC
```

After a successful execution, the program will exit. If the transaction fails, the program continues to stream new blocks.

3. **Skip this swap:** Ignores this swap. Then the program resumes listening for blocks.

:warning: **Important Note**

Market conditions can change rapidly. Delays in your decision-making can lead to transaction reverts, especially if you've set parameters like minimum amount out or slippage. Always ensure you're comfortable with the potential risks before executing swaps.

### Recap

In this quickstart, you explored how to use Tycho to:

1. **Connect to the Tycho Indexer**: Retrieve real-time protocol data filtered by TVL.
2. **Fetch Token and Pool Data**: Load all token details and process protocol updates.
3. **Simulate Token Swaps**: Compute the output amount, gas cost, and updated protocol state for a swap.
4. **Encode a Swap:** Create a solution from the best pool state and retrieve calldata to execute against a Tycho router.
5. **Execute a Swap:** Execute the best trade using the Tycho Router.

### What's next?



* **Integrate with your Solver**: Add Tycho pool liquidity to your solver, using this [guide](for-dexs/protocol-integration/).
* **Learn more about** [**Tycho Execution**](for-solvers/execution/) and the datatypes necessary to encode an execution against a Tycho router or executor.
* **Learn more about** [**Tycho Simulation**](for-solvers/simulation.md): Explore custom filters, protocol-specific simulations, and state transitions.
* **Explore** [**Tycho Indexer**](for-solvers/indexer/): Add or modify the data that Tycho indexes.
