//! HRTF Convolver — Rust implementation with C API
//!
//! This crate is compiled as a static library and linked into the C++ APO DLL.
//! The C++ code handles COM/APO boilerplate, this handles the DSP.

mod channel_map;
mod convolver;

pub use convolver::HrtfConvolver;
pub use channel_map::{ACTIVE_SPEAKERS, CHANNEL_TO_SPEAKER, LFE_GAIN};

// ── C API for the C++ APO to call ──────────────────────────────────────

/// Create a convolver from flat IR data.
/// `irs_flat`: 7 speakers × 2 ears × ir_length floats
/// Returns opaque pointer, or null on failure.
#[no_mangle]
pub extern "C" fn hrtf_convolver_create(irs_flat: *const f32, ir_length: u32) -> *mut HrtfConvolver {
    if irs_flat.is_null() || ir_length == 0 {
        return std::ptr::null_mut();
    }
    let total = ACTIVE_SPEAKERS * 2 * ir_length as usize;
    let slice = unsafe { std::slice::from_raw_parts(irs_flat, total) };
    let boxed = slice.to_vec().into_boxed_slice();
    let conv = HrtfConvolver::new(boxed, ir_length as usize);
    Box::into_raw(Box::new(conv))
}

/// Process audio: interleaved input (in_channels) → interleaved output (out_channels).
/// If in_channels == 8 and convolver is valid, applies HRTF convolution.
/// Otherwise passes through min(in_ch, out_ch) channels.
#[no_mangle]
pub extern "C" fn hrtf_convolver_process(
    conv: *mut HrtfConvolver,
    input: *const f32,
    output: *mut f32,
    n_frames: u32,
    in_channels: u32,
    out_channels: u32,
) {
    let n = n_frames as usize;
    let in_ch = in_channels as usize;
    let out_ch = out_channels as usize;

    if input.is_null() || output.is_null() || n == 0 {
        return;
    }

    let input_buf = unsafe { std::slice::from_raw_parts(input, n * in_ch) };
    let output_buf = unsafe { std::slice::from_raw_parts_mut(output, n * out_ch) };

    // HRTF convolution: only when 8ch input, 2ch output, and convolver exists
    if in_ch == 8 && out_ch == 2 && !conv.is_null() {
        let convolver = unsafe { &mut *conv };
        convolver.process(input_buf, output_buf, n);
    } else {
        // Passthrough: copy min(in_ch, out_ch) channels, zero the rest
        let copy_ch = in_ch.min(out_ch);
        for frame in 0..n {
            let in_base = frame * in_ch;
            let out_base = frame * out_ch;
            for ch in 0..copy_ch {
                output_buf[out_base + ch] = input_buf[in_base + ch];
            }
            for ch in copy_ch..out_ch {
                output_buf[out_base + ch] = 0.0;
            }
        }
    }
}

/// Process 8-channel interleaved audio in-place.
/// Writes binaural L/R to channels 0-1, zeros channels 2-7.
/// For non-8ch input or null convolver, buffer is left unchanged (passthrough).
#[no_mangle]
pub extern "C" fn hrtf_convolver_process_inplace(
    conv: *mut HrtfConvolver,
    buffer: *mut f32,
    n_frames: u32,
    channels: u32,
) {
    let n = n_frames as usize;
    let ch = channels as usize;

    if buffer.is_null() || n == 0 || conv.is_null() || ch < 2 {
        return;
    }

    if ch == 8 {
        let buf = unsafe { std::slice::from_raw_parts_mut(buffer, n * ch) };
        let convolver = unsafe { &mut *conv };
        convolver.process_inplace(buf, n);
    }
    // else: passthrough — leave buffer unchanged
}

/// Destroy a convolver.
#[no_mangle]
pub extern "C" fn hrtf_convolver_destroy(conv: *mut HrtfConvolver) {
    if !conv.is_null() {
        unsafe { drop(Box::from_raw(conv)); }
    }
}

/// Reset convolver delay lines.
#[no_mangle]
pub extern "C" fn hrtf_convolver_reset(conv: *mut HrtfConvolver) {
    if !conv.is_null() {
        unsafe { (*conv).reset(); }
    }
}

/// Load IR data from a binary file (C:\ProgramData\HrtfApo\hrtf_irs.bin).
/// Returns pointer to float array on success (caller must free with hrtf_free_irs).
/// Sets ir_length and sample_rate output params.
#[no_mangle]
pub extern "C" fn hrtf_load_irs(
    path: *const u16,
    out_ir_length: *mut u32,
    out_sample_rate: *mut u32,
) -> *mut f32 {
    if path.is_null() {
        return std::ptr::null_mut();
    }

    // Convert wide string to Rust path
    let path_slice = unsafe {
        let mut len = 0;
        while *path.add(len) != 0 { len += 1; }
        std::slice::from_raw_parts(path, len)
    };
    let path_str = String::from_utf16_lossy(path_slice);
    let path = std::path::Path::new(&path_str);

    match hrtf_synth::SpeakerIrSet::load(path) {
        Ok(ir_set) => {
            if !out_ir_length.is_null() {
                unsafe { *out_ir_length = ir_set.ir_length as u32; }
            }
            if !out_sample_rate.is_null() {
                unsafe { *out_sample_rate = ir_set.sample_rate; }
            }
            let mut data = ir_set.irs.into_boxed_slice();
            let ptr = data.as_mut_ptr();
            std::mem::forget(data); // caller frees via hrtf_free_irs
            ptr
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free IR data returned by hrtf_load_irs.
#[no_mangle]
pub extern "C" fn hrtf_free_irs(irs: *mut f32, count: u32) {
    if !irs.is_null() && count > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(irs, count as usize, count as usize);
        }
    }
}
