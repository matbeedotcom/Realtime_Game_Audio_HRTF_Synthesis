#pragma once

#include <audioenginebaseapo.h>
#include <BaseAudioProcessingObject.h>
#include <Unknwn.h>

// Forward declare the Rust FFI functions
extern "C" {
    void* hrtf_convolver_create(const float* irs_flat, unsigned int ir_length);
    void hrtf_convolver_process_inplace(void* conv, float* buffer,
                                        unsigned int n_frames, unsigned int channels);
    void hrtf_convolver_destroy(void* conv);
    void hrtf_convolver_reset(void* conv);
    float* hrtf_load_irs(const wchar_t* path, unsigned int* out_ir_length,
                         unsigned int* out_sample_rate);
    void hrtf_free_irs(float* irs, unsigned int count);
}

class INonDelegatingUnknown
{
    virtual HRESULT __stdcall NonDelegatingQueryInterface(const IID& iid, void** ppv) = 0;
    virtual ULONG __stdcall NonDelegatingAddRef() = 0;
    virtual ULONG __stdcall NonDelegatingRelease() = 0;
};

class HrtfApo : public CBaseAudioProcessingObject, public IAudioSystemEffects2, public INonDelegatingUnknown
{
public:
    HrtfApo(IUnknown* pUnkOuter);
    virtual ~HrtfApo();

    // IUnknown (delegates to pUnkOuter)
    virtual HRESULT __stdcall QueryInterface(const IID& iid, void** ppv);
    virtual ULONG __stdcall AddRef();
    virtual ULONG __stdcall Release();

    // IAudioProcessingObject
    virtual HRESULT __stdcall GetLatency(HNSTIME* pTime);
    virtual HRESULT __stdcall Initialize(UINT32 cbDataSize, BYTE* pbyData);
    virtual HRESULT __stdcall IsInputFormatSupported(IAudioMediaType* pOutputFormat,
        IAudioMediaType* pRequestedInputFormat, IAudioMediaType** ppSupportedInputFormat);

    // IAudioSystemEffects2
    virtual HRESULT __stdcall GetEffectsList(LPGUID* ppEffectsIds, UINT* pcEffects, HANDLE Event);

    // IAudioProcessingObjectConfiguration
    virtual HRESULT __stdcall LockForProcess(UINT32 u32NumInputConnections,
        APO_CONNECTION_DESCRIPTOR** ppInputConnections, UINT32 u32NumOutputConnections,
        APO_CONNECTION_DESCRIPTOR** ppOutputConnections);
    virtual HRESULT __stdcall UnlockForProcess(void);

    // IAudioProcessingObjectRT
    virtual void __stdcall APOProcess(UINT32 u32NumInputConnections,
        APO_CONNECTION_PROPERTY** ppInputConnections,
        UINT32 u32NumOutputConnections,
        APO_CONNECTION_PROPERTY** ppOutputConnections);

    // INonDelegatingUnknown
    virtual HRESULT __stdcall NonDelegatingQueryInterface(const IID& iid, void** ppv);
    virtual ULONG __stdcall NonDelegatingAddRef();
    virtual ULONG __stdcall NonDelegatingRelease();

    static long instCount;
    static const CRegAPOProperties<1> regProperties;

private:
    long refCount;
    IUnknown* pUnkOuter;
    void* rustConvolver;
    UINT32 inputChannels;
    UINT32 outputChannels;
    bool firstProcess;
};
