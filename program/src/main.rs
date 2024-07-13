//! A simple program that takes a number `n` as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use std::f64::consts::PI;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlackScholesInput {
    pub price: f64,
    pub strike: f64,
    pub iv: f64,
    pub time: f64,
    pub rate: f64,
}

fn norm_pdf(x: f64) -> f64 {
    (-x * x / 2.0).exp() / (2.0 * PI).sqrt()
}

fn norm_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989423 * (-x * x / 2.0).exp();
    let p = d * t * (0.3193815 + t * (-0.3565638 + t * (1.781478 + t * (-1.821256 + t * 1.330274))));
    if x > 0.0 { 1.0 - p } else { p }
}

fn d1(s: f64, k: f64, r: f64, sigma: f64, t: f64) -> f64 {
    (s.ln() / k + (r + sigma * sigma / 2.0) * t) / (sigma * t.sqrt())
}

fn d2(s: f64, k: f64, r: f64, sigma: f64, t: f64) -> f64 {
    d1(s, k, r, sigma, t) - sigma * t.sqrt()
}

fn call_price(s: f64, k: f64, r: f64, sigma: f64, t: f64) -> f64 {
    let d1 = d1(s, k, r, sigma, t);
    let d2 = d2(s, k, r, sigma, t);
    s * norm_cdf(d1) - k * (-r * t).exp() * norm_cdf(d2)
}

fn put_price(s: f64, k: f64, r: f64, sigma: f64, t: f64) -> f64 {
    let d1 = d1(s, k, r, sigma, t);
    let d2 = d2(s, k, r, sigma, t);
    k * (-r * t).exp() * norm_cdf(-d2) - s * norm_cdf(-d1)
}

pub fn main() {
    let option_input = sp1_zkvm::io::read::<BlackScholesInput>();

    let s = option_input.price; // Current stock price
    let k = option_input.strike; // Strike price
    let r = option_input.rate;  // Risk-free rate
    let sigma = option_input.iv; // Volatility
    let t = option_input.time;   // Time to expiration in years

    let call = call_price(s, k, r, sigma, t);
    let put = put_price(s, k, r, sigma, t);

    println!("Call option price: {:.4}", call);
    println!("Put option price: {:.4}", put);

    let call_bytes = bincode::serialize(&call).unwrap();
    let put_bytes = bincode::serialize(&put).unwrap();

    sp1_zkvm::io::commit_slice(&call_bytes);
    sp1_zkvm::io::commit_slice(&put_bytes);
}
