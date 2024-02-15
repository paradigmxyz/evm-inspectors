//! Fourbyte tracing inspector
//!
//! Solidity contract functions are addressed using the first four byte of the Keccak-256 hash of
//! their signature. Therefore when calling the function of a contract, the caller must send this
//! function selector as well as the ABI-encoded arguments as call data.
//!
//! The 4byteTracer collects the function selectors of every function executed in the lifetime of a
//! transaction, along with the size of the supplied call data. The result is a map of
//! SELECTOR-CALLDATASIZE to number of occurrences entries, where the keys are SELECTOR-CALLDATASIZE
//! and the values are number of occurrences of this key. For example:
//!
//! ```json
//! {
//!   "0x27dc297e-128": 1,
//!   "0x38cc4831-0": 2,
//!   "0x524f3889-96": 1,
//!   "0xadf59f99-288": 1,
//!   "0xc281d19e-0": 1
//! }
//! ```
//!
//! See also <https://geth.ethereum.org/docs/developers/evm-tracing/built-in-tracers>

use alloy_primitives::{hex, Selector};
use alloy_rpc_trace_types::geth::FourByteFrame;
use revm::{
    interpreter::{CallInputs, CallOutcome},
    Database, EvmContext, Inspector,
};
use std::{collections::HashMap, ops::Range};

/// Fourbyte tracing inspector that records all function selectors and their calldata sizes.
#[derive(Clone, Debug, Default)]
pub struct FourByteInspector {
    /// The map of SELECTOR to number of occurrences entries
    inner: HashMap<(Selector, usize), u64>,
}

impl FourByteInspector {
    /// Returns the map of SELECTOR to number of occurrences entries
    pub const fn inner(&self) -> &HashMap<(Selector, usize), u64> {
        &self.inner
    }
}

impl<DB> Inspector<DB> for FourByteInspector
where
    DB: Database,
{
    fn call(
        &mut self,
        _context: &mut EvmContext<DB>,
        inputs: &mut CallInputs,
        _return_memory_offset: Range<usize>,
    ) -> Option<CallOutcome> {
        if inputs.input.len() >= 4 {
            let selector =
                Selector::try_from(&inputs.input[..4]).expect("input is at least 4 bytes");
            let calldata_size = inputs.input[4..].len();
            *self.inner.entry((selector, calldata_size)).or_default() += 1;
        }

        None
    }
}

impl From<FourByteInspector> for FourByteFrame {
    fn from(value: FourByteInspector) -> Self {
        FourByteFrame(
            value
                .inner
                .into_iter()
                .map(|((selector, calldata_size), count)| {
                    let key = format!("0x{}-{}", hex::encode(&selector[..]), calldata_size);
                    (key, count)
                })
                .collect(),
        )
    }
}
