//! A simple program that takes a number `n` as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use std::f64::consts::PI;

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

pub fn main() {
    let s = 100.0; // Current stock price
    let k = 100.0; // Strike price
    let r = 0.05;  // Risk-free rate
    let sigma = 0.2; // Volatility
    let t = 1.0;   // Time to expiration in years

    let call = call_price(s, k, r, sigma, t);
    // let put = put_price(s, k, r, sigma, t);

    println!("Call option price: {:.4}", call);
    // println!("Put option price: {:.4}", put);

    // sp1_zkvm::io::write(&call);
    // sp1_zkvm::io::write(&b);
}

// pub fn main() {
//     // Read an input to the program.
//     //
//     // Behind the scenes, this compiles down to a custom system call which handles reading inputs
//     // from the prover.
//     let n = sp1_zkvm::io::read::<u32>();

//     // Write n to public input
//     sp1_zkvm::io::commit(&n);

//     // Compute the n'th fibonacci number, using normal Rust code.
//     let mut a = 0;
//     let mut b = 1;
//     for _ in 0..n {
//         let mut c = a + b;
//         c %= 7919; // Modulus to prevent overflow.
//         a = b;
//         b = c;
//     }

//     // Write the output of the program.
//     //
//     // Behind the scenes, this also compiles down to a custom system call which handles writing
//     // outputs to the prover.
//     sp1_zkvm::io::commit(&a);
//     sp1_zkvm::io::commit(&b);
// }
