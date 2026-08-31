//! Detached signing for Argorix evidence bundles.
//!
//! This is a separate binary on purpose. Offline verification establishes that
//! a bundle and its artifacts agree with each other, but not who produced
//! them, so a coordinated replacement of the whole set is indistinguishable
//! from the original. Signing closes that gap, and keeping the signer out of
//! the runtime means the VM never handles private key material.
//!
//! The keys this tool generates are for evaluation and local development. It
//! provides no key storage, rotation, revocation, or timestamping; a
//! deployment needs all four.

use anyhow::{bail, Context, Result};
use argorix_vm::evidence::{canonical_digest, EvidenceBundle};
use argorix_vm::signature::{
    encode_hex, read_key_file, signature_path, EvidenceSignature, SIGNATURE_ALGORITHM,
    SIGNATURE_VERSION,
};
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey};
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "argorix-sign",
    version,
    about = "Detached Ed25519 signing for Argorix evidence bundles"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate an evaluation key pair.
    Keygen {
        #[arg(long, value_name = "DIR")]
        out_dir: PathBuf,
        /// 32-byte hex seed. Supplying one makes the key reproducible, which a
        /// campaign needs; never do this for a key that protects anything.
        #[arg(long, value_name = "HEX")]
        seed: Option<String>,
    },
    /// Sign a bundle's canonical bytes, writing a detached signature.
    Sign {
        bundle: PathBuf,
        #[arg(long, value_name = "PATH")]
        key: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Keygen { out_dir, seed } => keygen(out_dir, seed),
        Command::Sign { bundle, key, out } => sign(bundle, key, out),
    }
}

fn keygen(out_dir: PathBuf, seed: Option<String>) -> Result<()> {
    let seed_bytes = match seed {
        Some(value) => decode_seed(&value)?,
        None => {
            let mut bytes = [0u8; 32];
            getrandom::fill(&mut bytes)
                .map_err(|error| anyhow::anyhow!("failed to read OS randomness: {error}"))?;
            bytes
        }
    };
    let signing = SigningKey::from_bytes(&seed_bytes);
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create `{}`", out_dir.display()))?;
    let private = out_dir.join("signing.key");
    let public = out_dir.join("verifying.key");
    fs::write(&private, format!("{}\n", encode_hex(&signing.to_bytes())))
        .with_context(|| format!("failed to write `{}`", private.display()))?;
    fs::write(
        &public,
        format!("{}\n", encode_hex(signing.verifying_key().as_bytes())),
    )
    .with_context(|| format!("failed to write `{}`", public.display()))?;
    println!("signing key:   {}", private.display());
    println!("verifying key: {}", public.display());
    println!("evaluation keys only: no storage, rotation or revocation");
    Ok(())
}

fn sign(bundle: PathBuf, key: PathBuf, out: Option<PathBuf>) -> Result<()> {
    let raw =
        fs::read(&bundle).with_context(|| format!("failed to read `{}`", bundle.display()))?;
    let parsed: EvidenceBundle = serde_json::from_slice(&raw)
        .with_context(|| format!("invalid evidence bundle in `{}`", bundle.display()))?;
    // Sign the canonical serialisation rather than the file bytes, so
    // reformatting the JSON does not invalidate the signature while any
    // semantic change does.
    let canonical = serde_json::to_vec(&parsed)?;
    let signing =
        SigningKey::from_bytes(&read_key_file(&key).map_err(|error| anyhow::anyhow!("{error}"))?);
    let signature = signing.sign(&canonical);

    let document = EvidenceSignature {
        signature_version: SIGNATURE_VERSION.into(),
        algorithm: SIGNATURE_ALGORITHM.into(),
        public_key: encode_hex(signing.verifying_key().as_bytes()),
        bundle_digest: canonical_digest(&parsed).map_err(|error| anyhow::anyhow!("{error}"))?,
        signature: encode_hex(&signature.to_bytes()),
    };
    let target = out.unwrap_or_else(|| signature_path(&bundle));
    let json = serde_json::to_string_pretty(&document)?;
    fs::write(&target, format!("{json}\n"))
        .with_context(|| format!("failed to write `{}`", target.display()))?;
    println!("signature written: {}", target.display());
    Ok(())
}

fn decode_seed(value: &str) -> Result<[u8; 32]> {
    let trimmed = value.trim();
    if trimmed.len() != 64 {
        bail!("seed must be 64 hex characters");
    }
    let mut bytes = [0u8; 32];
    for (index, slot) in bytes.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&trimmed[index * 2..index * 2 + 2], 16)
            .context("seed must be hex")?;
    }
    Ok(bytes)
}
