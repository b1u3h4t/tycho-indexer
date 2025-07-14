# Execution

To integrate a new protocol into Tycho, you need to implement two key components:

1. **SwapEncoder** (Rust struct) – Handles swap encoding.
2. **Executor** (Solidity contract) – Executes the swap on-chain.

See more about our code architecture [here](code-architecture.md).

## Main Encoder Interface

Each new protocol requires a dedicated `SwapEncoder` that implements the `SwapEncoder` trait. This trait defines how swaps for the protocol are encoded into calldata.

```rust
fn encode_swap(
    &self,
    swap: Swap,
    encoding_context: EncodingContext,
) -> Result<Vec<u8>, EncodingError>;
```

This function encodes a swap and its relevant context information into calldata that is compatible with the `Executor` contract. The output of the `SwapEncoder` is the input of the `Executor` (see next section). See current implementations [here](https://github.com/propeller-heads/tycho-execution/blob/main/src/encoding/evm/swap_encoder/swap_encoders.rs).

If your protocol needs some specific constant addresses please add them in [config/protocol\_specific\_addresses.json](https://github.com/propeller-heads/tycho-execution/blob/main/config/protocol_specific_addresses.json).

After implementing your `SwapEncoder` , you need to:

* Add your protocol with a placeholder address in: [config/executor\_addresses.json](https://github.com/propeller-heads/tycho-execution/blob/main/config/executor_addresses.json) and [config/test\_executor\_addresses.json](https://github.com/propeller-heads/tycho-execution/blob/main/config/test_executor_addresses.json)
* Add your protocol in the [`SwapEncoderBuilder`](https://github.com/propeller-heads/tycho-execution/blob/main/src/encoding/evm/swap_encoder/builder.rs#L36) .

<details>

<summary>Protocols Supporting Consecutive Swap Optimizations</summary>

As described in the [Swap Group](../../../for-solvers/execution/encoding.md#swap-group) section, our encoding supports protocols which save token transfers between consecutive swaps using systems such as flash accounting. In such cases, as shown in the diagram below using Uniswap V4 as an example, the `SwapEncoder` is still only in charge of encoding a **single swap**. These swaps will then be concatenated at the `StrategyEncoder` level as a single executor call.&#x20;

Depending on the index of the swap in the swap group, the encoder may be responsible for adding additional information which is not necessary in other swaps of the sequence (see the first swap in the diagram below).

<figure><img src="../../../.gitbook/assets/both (1).svg" alt=""><figcaption><p>Diagram representing swap groups</p></figcaption></figure>

<figure><img src="../../../.gitbook/assets/encoding data opt.svg" alt=""><figcaption><p>Output of a SwapEncoder for a group swap</p></figcaption></figure>

</details>

## Main Swap Interface

Every integrated protocol requires its own swap executor contract. This contract must conform to the `IExecutor` interface, allowing it to interact with the protocol and perform swaps be leveraging the `RestrictTransferFrom` contract. See currently implemented executors [here](https://github.com/propeller-heads/tycho-execution/tree/main/foundry/src/executors).

The `IExecutor` interface has the main method:

```solidity
function swap(uint256 givenAmount, bytes calldata data)
    external
    payable
    returns (uint256 calculatedAmount)
{
```

This function:

* Accepts the input amount (`givenAmount`). Note that the input amount is calculated at execution time and not during encoding. This is to account for possible slippage.
* Processes the swap using the provided calldata (`data`) which is the output of the `SwapEncoder`.
* Returns the final output amount (`calculatedAmount`).

Ensure that the implementation supports transferring received tokens to a designated receiver address, either within the swap function or through an additional transfer step.

If the protocol requires token approvals (allowances) before swaps can occur, manage these approvals within the implementation to ensure smooth execution of the swap.

### Callbacks

Some protocols require a callback during swap execution. In these cases, the executor contract must inherit from [`ICallback`](https://github.com/propeller-heads/tycho-execution/blob/main/foundry/interfaces/ICallback.sol) and implement the necessary callback functions.

**Required Methods**

```solidity
function handleCallback(
    bytes calldata data
) external returns (bytes memory result);

function verifyCallback(bytes calldata data) external view;
```

* `handleCallback`: The main entry point for handling callbacks.
* `verifyCallback`: Should be called within `handleCallback` to ensure that the `msg.sender` is a valid pool from the expected protocol.

## Token Transfers

The **Executor** contracts manage token transfers between the user, protocols, and the Tycho Router. The only exception is when unwrapping WETH to ETH after a swap—in this case, the router performs the final transfer to the receiver.

The `TychoRouter` architecture optimizes token transfers and reduces gas costs during both single and sequential swaps. Whenever possible:

* The executor transfers input tokens directly from the user to the target protocol.
* The executor instructs the protocol to send output tokens directly to the next protocol in the swap sequence.
* For the final swap in a sequence, the protocol sends output tokens directly to the user.

Each executor must inherit from the `RestrictTransferFrom` contract, which enables flexible and safe transfer logic. During encoding, the executor receives instructions specifying one of the following transfer types:

| Transfer Type   | Description                                                                                                                    |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `TRANSFER_FROM` | Transfers tokens from the user's wallet into the `TychoRouter` or into the pool. It can use permit2 or normal token transfers. |
| `TRANSFER`      | Assumes funds are already in the `TychoRouter` and transfers tokens into the pool                                              |
| `NONE`          | Assumes tokens are already in place for the swap; no transfer action is taken.                                                 |

Two key constants are used in encoding to configure protocol-specific behavior:

| Constant                         | Description                                                                                                                                                                                                                                              |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `IN_TRANSFER_REQUIRED_PROTOCOLS` | A list of protocols that require tokens to be transferred into the pool prior to swapping. These protocols do not perform a `transferFrom` during the `swap` themselves, and therefore require tokens to be transferred beforehand or during a callback. |
| `CALLBACK_CONSTRAINED_PROTOCOLS` | Protocols that require owed tokens to be transferred during a callback. In these cases, tokens cannot be transferred directly from the previous pool before the current swap begins.                                                                     |

Include your protocol in these constants if necessary.

## Testing

Each new integration must be thoroughly tested in both Rust and Solidity. This includes:

* Unit tests for the `SwapEncoder` in Rust
* Unit tests for the `Executor` in Solidity
* Two key **integration tests** to verify the full swap flow: `SwapEncoder` to `Executor` integration test and a full TychoRouter integration test

#### 1. `SwapEncoder` ↔ `Executor` integration test

Verify that the calldata generated by the `SwapEncoder` is accepted by the corresponding `Executor`.

Use the helper functions:

* `write_calldata_to_file()` in the encoding module (Rust)
* `loadCallDataFromFile()` in the execution module (Solidity)

These helpers save and load the calldata to/from `calldata.txt`.

#### 2. Full TychoRouter Integration Test

* In `tests/protocol_integration_tests.rs`, write a Rust test that encodes a single swap and saves the calldata using `write_calldata_to_file()`.
* In `TychoRouterTestSetup`, deploy your new executor and add it to executors list in `deployExecutors`.
* Run the setup to retrieve your executor’s deployed address and add it to [config/test\_executor\_addresses.json](https://github.com/propeller-heads/tycho-execution/blob/main/config/test_executor_addresses.json).
* Create a new Solidity test contract that inherits from `TychoRouterTestSetup`. For example:

```solidity
 contract TychoRouterForYouProtocolTest is TychoRouterTestSetup {
    function getForkBlock() public pure override returns (uint256) {
        return 22644371; // Use a block that fits your test scenario
    }

    function testSingleYourProtocolIntegration() public {
        ...
    }
}
```

These tests ensure your integration works end-to-end within Tycho’s architecture.

## Deploying and Whitelisting

Once your implementation is approved:

1. **Deploy the executor contract** on the appropriate network.
2. **Contact us** to whitelist the new executor address on our main router contract.
3. **Update the configuration** by adding the new executor address to `executor_addresses.json` and register the `SwapEncoder` within the `SwapEncoderBuilder` .

By following these steps, your protocol will be fully integrated with Tycho, enabling it to execute swaps seamlessly.
