//! silence-engine — deterministic scheduler. CPU-only. No ML. No system clock.
#![cfg_attr(feature = "wasm", no_std)]
extern crate alloc;
use alloc::vec::Vec;
use core::fmt;
pub mod hash;
#[cfg(feature = "wasm")]
pub mod wasm;
pub const GOLDENSECOND: u64 = 1618;
pub const PHI_INV_NUM: u64 = 618_033_988_749_894;
pub const PHI_INV_DEN: u64 = 1_000_000_000_000_000;
pub const MAX_SLOTS_PER_CYCLE: usize = 3;
pub const MAX_SCHEDULE_SPAN_MS: u64 = 5000;
pub type ObserverPseudonym = [u8; 16];
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum AttentionDepth { Surface = 1, Shallow = 2, Moderate = 3, Deep = 4, Flow = 5 }
impl AttentionDepth {
    pub fn from_raw(v: u8) -> Option<Self> {
        match v { 1 => Some(Self::Surface), 2 => Some(Self::Shallow), 3 => Some(Self::Moderate), 4 => Some(Self::Deep), 5 => Some(Self::Flow), _ => None }
    }
    pub fn to_raw(self) -> u8 { self as u8 }
}
impl core::convert::TryFrom<u8> for AttentionDepth {
    type Error = &'static str;
    fn try_from(v: u8) -> Result<Self, Self::Error> { Self::from_raw(v).ok_or("invalid attention depth") }
}
impl From<AttentionDepth> for u8 { fn from(d: AttentionDepth) -> u8 { d.to_raw() } }
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EngineInput {
    pub observer: ObserverPseudonym,
    pub timestamp_ms: u64,
    pub attention_depth: AttentionDepth,
    pub last_signal_ms: Option<u64>,
    pub entropy: [u8; 8],
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct SignalSlot { pub scheduled_ms: u64, pub priority: u8, pub signal_id: [u8; 16] }
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EngineOutput { pub slots: Vec<SignalSlot>, pub input_hash: [u8; 32], pub seed: u64, pub output_hash: [u8; 32] }
impl EngineInput {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(42);
        buf.extend_from_slice(&self.observer);
        buf.extend_from_slice(&self.timestamp_ms.to_le_bytes());
        buf.push(self.attention_depth.to_raw());
        match self.last_signal_ms {
            Some(t) => { buf.push(1); buf.extend_from_slice(&t.to_le_bytes()); }
            None => { buf.push(0); buf.extend_from_slice(&0u64.to_le_bytes()); }
        }
        buf.extend_from_slice(&self.entropy);
        buf
    }
}
impl SignalSlot {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(25);
        buf.extend_from_slice(&self.scheduled_ms.to_le_bytes());
        buf.push(self.priority);
        buf.extend_from_slice(&self.signal_id);
        buf
    }
}
pub fn compute_input_hash(input: &EngineInput) -> [u8; 32] { hash::sha256(&input.to_bytes()) }
pub fn derive_seed(input_hash: &[u8; 32]) -> u64 {
    let mut bytes = [0u8; 8]; bytes.copy_from_slice(&input_hash[0..8]); u64::from_le_bytes(bytes)
}
pub fn compute_output_hash(seed: u64, slots: &[SignalSlot]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(8 + slots.len() * 25);
    buf.extend_from_slice(&seed.to_le_bytes());
    for slot in slots { buf.extend_from_slice(&slot.to_bytes()); }
    hash::sha256(&buf)
}
fn compute_interval_ms(depth: AttentionDepth) -> u64 {
    let n = depth.to_raw() as u64; let mut interval = GOLDENSECOND;
    for _ in 0..n { interval = (interval * PHI_INV_NUM) / PHI_INV_DEN; }
    interval
}
fn deterministic_signal_id(seed: u64, index: u32) -> [u8; 16] {
    let mut buf = [0u8; 24];
    buf[0..8].copy_from_slice(&seed.to_le_bytes());
    buf[8..12].copy_from_slice(&index.to_le_bytes());
    buf[12..16].copy_from_slice(b"SILO"); buf[16..20].copy_from_slice(b"_ENG"); buf[20..24].copy_from_slice(b"_001");
    let h = hash::sha256(&buf); let mut out = [0u8; 16]; out.copy_from_slice(&h[0..16]); out
}
fn priority_from_depth(depth: AttentionDepth) -> u8 {
    match depth { AttentionDepth::Surface | AttentionDepth::Shallow => 3, AttentionDepth::Moderate | AttentionDepth::Deep => 2, AttentionDepth::Flow => 1 }
}
pub fn compute_schedule(input: &EngineInput) -> EngineOutput {
    let input_hash = compute_input_hash(input); let seed = derive_seed(&input_hash);
    let interval = compute_interval_ms(input.attention_depth); let base_time = input.timestamp_ms;
    let count = MAX_SLOTS_PER_CYCLE.min(3); let mut slots = Vec::with_capacity(count);
    for i in 0..count {
        slots.push(SignalSlot { scheduled_ms: base_time + interval * (i as u64 + 1), priority: priority_from_depth(input.attention_depth), signal_id: deterministic_signal_id(seed, i as u32) });
    }
    slots.sort_by_key(|s| s.scheduled_ms);
    let output_hash = compute_output_hash(seed, &slots);
    EngineOutput { slots, input_hash, seed, output_hash }
}
pub fn validate_output(input: &EngineInput, output: &EngineOutput) -> Result<(), ValidationError> {
    if output.input_hash != compute_input_hash(input) { return Err(ValidationError::InputHashMismatch); }
    if output.seed != derive_seed(&output.input_hash) { return Err(ValidationError::SeedMismatch); }
    if output.output_hash != compute_output_hash(output.seed, &output.slots) { return Err(ValidationError::OutputHashMismatch); }
    if output.slots.len() > MAX_SLOTS_PER_CYCLE { return Err(ValidationError::TooManySlots); }
    for i in 1..output.slots.len() { if output.slots[i].scheduled_ms < output.slots[i - 1].scheduled_ms { return Err(ValidationError::SlotsNotSorted); } }
    if let Some(last) = output.slots.last() { if last.scheduled_ms.saturating_sub(input.timestamp_ms) > MAX_SCHEDULE_SPAN_MS { return Err(ValidationError::SpanExceeded); } }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationError { InputHashMismatch, SeedMismatch, OutputHashMismatch, TooManySlots, SlotsNotSorted, SpanExceeded }
impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InputHashMismatch => write!(f, "ENGINE_INPUT_HASH_MISMATCH"),
            ValidationError::SeedMismatch => write!(f, "ENGINE_SEED_MISMATCH"),
            ValidationError::OutputHashMismatch => write!(f, "ENGINE_OUTPUT_HASH_MISMATCH"),
            ValidationError::TooManySlots => write!(f, "ENGINE_TOO_MANY_SLOTS"),
            ValidationError::SlotsNotSorted => write!(f, "ENGINE_SLOTS_NOT_SORTED"),
            ValidationError::SpanExceeded => write!(f, "ENGINE_SPAN_EXCEEDED"),
        }
    }
}
impl core::error::Error for ValidationError {}
pub fn compute_batch(inputs: &[EngineInput]) -> Vec<EngineOutput> { inputs.iter().map(compute_schedule).collect() }
pub fn verify_determinism(inputs: &[EngineInput]) -> bool { compute_batch(inputs) == compute_batch(inputs) }
