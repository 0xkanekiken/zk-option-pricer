use sp1_sdk::{utils, ProverClient, SP1Proof, SP1Stdin};
use serde::{Serialize, Deserialize};

/// The ELF we want to execute inside the zkVM.
const ELF: &[u8] = include_bytes!("../../program/elf/riscv32im-succinct-zkvm-elf");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlackScholesInput {
    pub price: f64,
    pub strike: f64,
    pub iv: f64,
    pub time: f64,
    pub rate: f64,
}

impl Default for BlackScholesInput {
    fn default() -> Self {
        BlackScholesInput {
            price: 100.0,
            strike: 105.0,
            iv: 0.2,
            time: 1.0,
            rate: 0.05,
        }
    }
}

fn main() {
    // Generate proof.
    // utils::setup_tracer();
    utils::setup_logger();

    let input = BlackScholesInput::default();
    let mut stdin = SP1Stdin::new();
    stdin.write(&input);

    let client = ProverClient::new();
    let (pk, vk) = client.setup(ELF);
    let proof = client.prove(&pk, stdin).expect("proving failed");

    // Verify proof.
    client.verify(&proof, &vk).expect("verification failed");

    // Test a round trip of proof serialization and deserialization.
    proof
        .save("proof-with-pis.bin")
        .expect("saving proof failed");
    let deserialized_proof = SP1Proof::load("proof-with-pis.bin").expect("loading proof failed");

    // Verify the deserialized proof.
    client
        .verify(&deserialized_proof, &vk)
        .expect("verification failed");

    println!("successfully generated and verified proof for the program!")
}
