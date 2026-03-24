#pragma once

// C API exported by the Rust static library
#ifdef __cplusplus
extern "C" {
#endif

void* hrtf_convolver_create(const float* irs_flat, unsigned int ir_length);
void hrtf_convolver_process(void* conv, const float* input, float* output,
                            unsigned int n_frames, unsigned int in_channels,
                            unsigned int out_channels);
void hrtf_convolver_process_inplace(void* conv, float* buffer,
                                    unsigned int n_frames, unsigned int channels);
void hrtf_convolver_destroy(void* conv);
void hrtf_convolver_reset(void* conv);

// Load IR data from a UTF-16 file path. Returns float array (free with hrtf_free_irs).
float* hrtf_load_irs(const wchar_t* path, unsigned int* out_ir_length,
                     unsigned int* out_sample_rate);
void hrtf_free_irs(float* irs, unsigned int count);

#ifdef __cplusplus
}
#endif
