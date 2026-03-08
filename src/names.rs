use std::time::{SystemTime, UNIX_EPOCH};

// ~20 words per theme: package/delivery, code/dev, AI
const WORDS: &[&str] = &[
    // package & delivery
    "parcel", "cargo", "crate", "bundle", "shipment",
    "pallet", "depot", "courier", "dispatch", "manifest",
    "label", "transit", "express", "freight", "hub",
    "route", "packet", "pod", "bay", "silo",
    // code & dev
    "commit", "branch", "patch", "deploy", "build",
    "merge", "stack", "kernel", "daemon", "buffer",
    "queue", "cache", "socket", "hook", "pipeline",
    "artifact", "release", "index", "token", "module",
    // AI
    "neural", "tensor", "gradient", "epoch", "inference",
    "prompt", "embedding", "agent", "vector", "cluster",
    "layer", "attention", "weight", "model", "feature",
    "sample", "batch", "node", "matrix", "signal",
];

/// Generate a memorable three-word name using a simple LCG seeded from the
/// current time. Not cryptographically random, but plenty of variety for
/// workspace names.
pub fn random_name() -> String {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(42);

    let n = WORDS.len() as u128;

    // Three independent LCG steps (different multipliers to reduce correlation)
    let a = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) >> 33) as usize % n as usize;
    let b = ((seed.wrapping_mul(2862933555777941757).wrapping_add(3037000499)) >> 33) as usize % n as usize;
    let c = ((seed.wrapping_mul(1442695040888963407).wrapping_add(6364136223846793005)) >> 33) as usize % n as usize;

    format!("{}-{}-{}", WORDS[a], WORDS[b], WORDS[c])
}
