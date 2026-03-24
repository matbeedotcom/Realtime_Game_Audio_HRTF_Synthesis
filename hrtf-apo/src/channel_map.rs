//! Maps WAVEFORMATEXTENSIBLE 7.1 channel indices to HRTF speaker IR indices.
//!
//! Windows 7.1 channel order (WAVEFORMATEXTENSIBLE):
//!   0: FL  (Front Left)
//!   1: FR  (Front Right)
//!   2: FC  (Front Center)
//!   3: LFE (Low Frequency Effects) — handled separately, no HRTF
//!   4: RL  (Rear Left)
//!   5: RR  (Rear Right)
//!   6: SL  (Side Left)
//!   7: SR  (Side Right)
//!
//! HeSuVi speaker order (from hrtf-synth DEFAULT_SPEAKERS):
//!   0: FR  (30°)    → Windows ch 1
//!   1: SR  (110°)   → Windows ch 7
//!   2: RR  (150°)   → Windows ch 5
//!   3: FC  (0°)     → Windows ch 2
//!   4: FL  (330°)   → Windows ch 0
//!   5: SL  (250°)   → Windows ch 6
//!   6: RL  (210°)   → Windows ch 4

/// Number of active speaker channels (7.1 minus LFE)
pub const ACTIVE_SPEAKERS: usize = 7;

/// Maps our internal speaker index (0..7) to the WAVEFORMATEXTENSIBLE input channel index.
/// Order matches hrtf-synth DEFAULT_SPEAKERS: FR, SR, RR, FC, FL, SL, RL
pub const CHANNEL_TO_SPEAKER: [usize; 7] = [
    1, // speaker 0 (FR, 30°)  ← Windows ch 1
    7, // speaker 1 (SR, 110°) ← Windows ch 7
    5, // speaker 2 (RR, 150°) ← Windows ch 5
    2, // speaker 3 (FC, 0°)   ← Windows ch 2
    0, // speaker 4 (FL, 330°) ← Windows ch 0
    6, // speaker 5 (SL, 250°) ← Windows ch 6
    4, // speaker 6 (RL, 210°) ← Windows ch 4
];

/// LFE mixing gain (attenuate sub-bass slightly to avoid clipping)
pub const LFE_GAIN: f32 = 0.5;
