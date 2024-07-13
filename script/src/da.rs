use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use bytes::Bytes;
use sp_core::H256;
use tracing::info;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use avail_subxt::{
    api::runtime_types::{da_control::pallet::Call, da_runtime::RuntimeCall::DataAvailability},
    primitives::AppUncheckedExtrinsic,
};
use codec::Encode;
use anyhow::anyhow;
use avail_subxt::api::runtime_types::sp_core::bounded::bounded_vec::BoundedVec;
use avail_subxt::primitives::AvailExtrinsicParams;
use avail_subxt::{api, AvailConfig};
use sp_core::crypto::Pair as PairTrait;
use sp_keyring::sr25519::sr25519::Pair;
use subxt::tx::PairSigner;
use subxt::OnlineClient;

#[derive(Debug, Clone, Serialize, Deserialize, BorshDeserialize, BorshSerialize, PartialEq)]
/// Simple structure that implements the Read trait for a buffer and  counts the number of bytes read from the beginning.
/// Useful for the partial blob reading optimization: we know for each blob how many bytes have been read from the beginning.
///
/// Because of soundness issues we cannot implement the Buf trait because the prover could get unproved blob data using the chunk method.
pub struct CountedBufReader<B: bytes::Buf> {
    /// The original blob data.
    inner: B,

    /// An accumulator that stores the data read from the blob buffer into a vector.
    /// Allows easy access to the data that has already been read
    accumulator: Vec<u8>,
}

impl<B: bytes::Buf> CountedBufReader<B> {
    /// Creates a new buffer reader with counter from an objet that implements the buffer trait
    pub fn new(inner: B) -> Self {
        let buf_size = inner.remaining();
        CountedBufReader {
            inner,
            accumulator: Vec::with_capacity(buf_size),
        }
    }

    /// Advance the accumulator by `num_bytes` bytes. If `num_bytes` is greater than the length
    /// of remaining unverified data, then all remaining unverified data is added to the accumulator.
    pub fn advance(&mut self, num_bytes: usize) {
        let requested = num_bytes;
        let remaining = self.inner.remaining();
        if remaining == 0 {
            return;
        }
        // `Buf::advance` would panic if `num_bytes` was greater than the length of the remaining unverified data,
        // but we just advance to the end of the buffer.
        let num_to_read = core::cmp::min(remaining, requested);
        // Extend the inner vector with zeros (copy_to_slice requires the vector to have
        // the correct *length* not just capacity)
        self.accumulator
            .resize(self.accumulator.len() + num_to_read, 0);

        // Use copy_to_slice to overwrite the zeros we just added
        let accumulator_len = self.accumulator.len();
        self.inner
            .copy_to_slice(self.accumulator[accumulator_len - num_to_read..].as_mut());
    }

    /// Getter: returns a reference to an accumulator of the blob data read by the rollup
    pub fn accumulator(&self) -> &[u8] {
        &self.accumulator
    }

    /// Contains the total length of the data (length already read + length remaining)
    pub fn total_len(&self) -> usize {
        self.inner.remaining() + self.accumulator.len()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct AvailAddress([u8; 32]);

impl Display for AvailAddress {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let hash = H256(self.0);
        write!(f, "{hash}")
    }
}

impl AsRef<[u8]> for AvailAddress {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<[u8; 32]> for AvailAddress {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl FromStr for AvailAddress {
    type Err = <H256 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let h_256 = H256::from_str(s)?;

        Ok(Self(h_256.to_fixed_bytes()))
    }
}

impl<'a> TryFrom<&'a [u8]> for AvailAddress {
    type Error = anyhow::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Self(<[u8; 32]>::try_from(value)?))
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AvailBlobTransaction {
    blob: CountedBufReader<Bytes>,
    hash: [u8; 32],
    address: AvailAddress,
}

impl AvailBlobTransaction {
    pub fn new(unchecked_extrinsic: &AppUncheckedExtrinsic) -> anyhow::Result<Self> {
        let address = match &unchecked_extrinsic.signature {
            //TODO: Handle other types of MultiAddress.
            Some((subxt::utils::MultiAddress::Id(id), _, _)) => AvailAddress::from(id.clone().0),
            _ => {
                return Err(anyhow!(
                    "Unsigned extrinsic being used to create AvailBlobTransaction."
                ))
            }
        };
        let blob = match &unchecked_extrinsic.function {
            DataAvailability(Call::submit_data { data }) => {
                CountedBufReader::<Bytes>::new(Bytes::copy_from_slice(&data.0))
            }
            _ => {
                return Err(anyhow!(
                    "Invalid type of extrinsic being converted to AvailBlobTransaction."
                ))
            }
        };

        Ok(AvailBlobTransaction {
            hash: sp_core_hashing::blake2_256(&unchecked_extrinsic.encode()),
            address,
            blob,
        })
    }

    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    pub fn combine_hash(&self, hash: [u8; 32]) -> [u8; 32] {
        let mut combined_hashes: Vec<u8> = Vec::with_capacity(64);
        combined_hashes.extend_from_slice(hash.as_ref());
        combined_hashes.extend_from_slice(self.hash().as_ref());

        sp_core_hashing::blake2_256(&combined_hashes)
    }
}

/// Runtime configuration for the DA service
#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DaServiceConfig {
    pub node_client_url: String,
    //TODO: Safer strategy to load seed so it is not accidentally revealed.
    pub seed: String,
    pub app_id: u32,
}

#[derive(Clone)]
pub struct DaProvider {
    pub node_client: OnlineClient<AvailConfig>,
    signer: PairSigner<AvailConfig, Pair>,
    app_id: u32,
}

impl DaProvider {

    pub async fn new(config: DaServiceConfig) -> Self {
        let pair = Pair::from_string_with_seed(&config.seed, None).unwrap();
        let signer = PairSigner::<AvailConfig, Pair>::new(pair.0.clone());

        let node_client = avail_subxt::build_client(config.node_client_url.to_string(), false)
            .await
            .unwrap();

        DaProvider {
            node_client,
            signer,
            app_id: config.app_id,
        }
    }
}

pub async fn send_transaction(da_provider: &DaProvider, blob: &[u8]) -> Result<(), anyhow::Error> {
    let data_transfer = api::tx()
        .data_availability()
        .submit_data(BoundedVec(blob.to_vec()));

    let extrinsic_params = AvailExtrinsicParams::new_with_app_id(da_provider.app_id.into());

    let h = da_provider
        .node_client
        .tx()
        .sign_and_submit_then_watch(&data_transfer, &da_provider.signer, extrinsic_params)
        .await?;

    println!("Transaction submitted");

    info!("Transaction submitted: {:#?}", h.extrinsic_hash());

    Ok(())
}