#include "HrtfApo.h"
#include <initguid.h>
#include "guids.h"
#include <audioengineextensionapo.h>
#include <cstdio>
#include <cstring>

static void DebugLog(const char* msg)
{
    FILE* f = nullptr;
    fopen_s(&f, "C:\\ProgramData\\HrtfApo\\debug.log", "a");
    if (f) { fprintf(f, "[HrtfApo] %s\r\n", msg); fclose(f); }
}

static void DebugLogf(const char* fmt, ...)
{
    char buf[512];
    va_list args;
    va_start(args, fmt);
    vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);
    DebugLog(buf);
}

// ── Static registration properties ──────────────────────────────────────
// Matches EqualizerAPO's flags exactly: FRAMESPERSECOND_MUST_MATCH | BITSPERSAMPLE_MUST_MATCH | INPLACE

long HrtfApo::instCount = 0;
const CRegAPOProperties<1> HrtfApo::regProperties(
    CLSID_HrtfApo,
    L"HRTF Binaural Audio",
    L"Copyright (C) 2026",
    1, 0,
    __uuidof(IAudioProcessingObject),
    (APO_FLAG)(APO_FLAG_FRAMESPERSECOND_MUST_MATCH | APO_FLAG_BITSPERSAMPLE_MUST_MATCH | APO_FLAG_INPLACE)
);

// ── Constructor / Destructor ────────────────────────────────────────────

HrtfApo::HrtfApo(IUnknown* pUnkOuter)
    : CBaseAudioProcessingObject(regProperties)
{
    refCount = 1;
    if (pUnkOuter != NULL)
        this->pUnkOuter = pUnkOuter;
    else
        this->pUnkOuter = reinterpret_cast<IUnknown*>(static_cast<INonDelegatingUnknown*>(this));

    rustConvolver = nullptr;
    inputChannels = 0;
    outputChannels = 0;
    firstProcess = true;

    InterlockedIncrement(&instCount);
    DebugLog("HrtfApo constructed");
}

HrtfApo::~HrtfApo()
{
    InterlockedDecrement(&instCount);
    if (rustConvolver) {
        hrtf_convolver_destroy(rustConvolver);
        rustConvolver = nullptr;
    }
    DebugLog("HrtfApo destroyed");
}

// ── IUnknown (delegating) ───────────────────────────────────────────────

HRESULT HrtfApo::QueryInterface(const IID& iid, void** ppv)
{
    return pUnkOuter->QueryInterface(iid, ppv);
}

ULONG HrtfApo::AddRef()
{
    return pUnkOuter->AddRef();
}

ULONG HrtfApo::Release()
{
    return pUnkOuter->Release();
}

// ── INonDelegatingUnknown ───────────────────────────────────────────────

HRESULT HrtfApo::NonDelegatingQueryInterface(const IID& iid, void** ppv)
{
    if (iid == __uuidof(IUnknown))
        *ppv = static_cast<INonDelegatingUnknown*>(this);
    else if (iid == __uuidof(IAudioProcessingObject))
        *ppv = static_cast<IAudioProcessingObject*>(this);
    else if (iid == __uuidof(IAudioProcessingObjectRT))
        *ppv = static_cast<IAudioProcessingObjectRT*>(this);
    else if (iid == __uuidof(IAudioProcessingObjectConfiguration))
        *ppv = static_cast<IAudioProcessingObjectConfiguration*>(this);
    else if (iid == __uuidof(IAudioSystemEffects))
        *ppv = static_cast<IAudioSystemEffects2*>(this);
    else if (iid == __uuidof(IAudioSystemEffects2))
        *ppv = static_cast<IAudioSystemEffects2*>(this);
    else
    {
        *ppv = NULL;
        char buf[128];
        snprintf(buf, sizeof(buf), "NonDelegatingQI FAILED: %08lX-%04X-%04X",
                 iid.Data1, iid.Data2, iid.Data3);
        DebugLog(buf);
        return E_NOINTERFACE;
    }

    reinterpret_cast<IUnknown*>(*ppv)->AddRef();
    return S_OK;
}

ULONG HrtfApo::NonDelegatingAddRef()
{
    return InterlockedIncrement(&refCount);
}

ULONG HrtfApo::NonDelegatingRelease()
{
    if (InterlockedDecrement(&refCount) == 0)
    {
        delete this;
        return 0;
    }
    return refCount;
}

// ── IAudioProcessingObject ──────────────────────────────────────────────

HRESULT HrtfApo::Initialize(UINT32 cbDataSize, BYTE* pbyData)
{
    DebugLogf("Initialize called, dataSize=%u", cbDataSize);

    if (NULL == pbyData && 0 != cbDataSize) return E_INVALIDARG;
    if (NULL != pbyData && 0 == cbDataSize) return E_INVALIDARG;

    // Handle all three init struct versions (matches SYSVAD sample pattern)
    if (cbDataSize == sizeof(APOInitSystemEffects3))
    {
        DebugLog("Initialize v3");
    }
    else if (cbDataSize == sizeof(APOInitSystemEffects2))
    {
        APOInitSystemEffects2* init2 = (APOInitSystemEffects2*)pbyData;
        DebugLogf("Initialize v2, CLSID=%08lX", init2->APOInit.clsid.Data1);
    }
    else if (cbDataSize == sizeof(APOInitSystemEffects))
    {
        APOInitSystemEffects* init1 = (APOInitSystemEffects*)pbyData;
        DebugLogf("Initialize v1, CLSID=%08lX", init1->APOInit.clsid.Data1);
    }
    else
    {
        DebugLogf("Initialize FAILED: unexpected size %u", cbDataSize);
        return E_INVALIDARG;
    }

    // Call base class Initialize with the v1 size (base class only reads APOInitBaseStruct)
    HRESULT hr = CBaseAudioProcessingObject::Initialize(cbDataSize, pbyData);
    DebugLogf("Base Initialize: 0x%08lX", hr);
    if (FAILED(hr)) return hr;

    // Load IRs
    const wchar_t* irPath = L"C:\\ProgramData\\HrtfApo\\hrtf_irs.bin";
    UINT32 irLength = 0, sampleRate = 0;
    float* irs = hrtf_load_irs(irPath, &irLength, &sampleRate);
    if (irs) {
        DebugLogf("IRs loaded: length=%u, sampleRate=%u", irLength, sampleRate);
        if (rustConvolver) hrtf_convolver_destroy(rustConvolver);
        rustConvolver = hrtf_convolver_create(irs, irLength);
        hrtf_free_irs(irs, 7 * 2 * irLength);
        DebugLogf("Convolver created: %p", rustConvolver);
    } else {
        DebugLog("No IRs found — passthrough mode");
    }

    return S_OK;
}

HRESULT HrtfApo::GetLatency(HNSTIME* pTime)
{
    if (!pTime) return E_POINTER;
    if (!m_bIsLocked) return APOERR_ALREADY_UNLOCKED;
    *pTime = 0;
    return S_OK;
}

HRESULT HrtfApo::GetEffectsList(LPGUID* ppEffectsIds, UINT* pcEffects, HANDLE Event)
{
    if (!ppEffectsIds || !pcEffects) return E_POINTER;
    *ppEffectsIds = NULL;
    *pcEffects = 0;
    return S_OK;
}

HRESULT HrtfApo::IsInputFormatSupported(
    IAudioMediaType* pOutputFormat,
    IAudioMediaType* pRequestedInputFormat,
    IAudioMediaType** ppSupportedInputFormat)
{
    if (!pRequestedInputFormat || !ppSupportedInputFormat) return E_POINTER;

    UNCOMPRESSEDAUDIOFORMAT inFormat;
    HRESULT hr = pRequestedInputFormat->GetUncompressedAudioFormat(&inFormat);
    if (FAILED(hr)) return hr;

    DebugLogf("IsInputFormatSupported: ch=%u, rate=%.0f, bps=%u",
              inFormat.dwSamplesPerFrame, inFormat.fFramesPerSecond,
              inFormat.dwValidBitsPerSample);

    // Accept any float32 format — we handle any channel count in APOProcess
    *ppSupportedInputFormat = pRequestedInputFormat;
    (*ppSupportedInputFormat)->AddRef();
    return S_OK;
}

// ── IAudioProcessingObjectConfiguration ─────────────────────────────────

HRESULT HrtfApo::LockForProcess(
    UINT32 u32NumInputConnections,
    APO_CONNECTION_DESCRIPTOR** ppInputConnections,
    UINT32 u32NumOutputConnections,
    APO_CONNECTION_DESCRIPTOR** ppOutputConnections)
{
    DebugLogf("LockForProcess: %u inputs, %u outputs", u32NumInputConnections, u32NumOutputConnections);

    UNCOMPRESSEDAUDIOFORMAT inFormat = {};
    if (ppInputConnections[0]->pFormat) {
        ppInputConnections[0]->pFormat->GetUncompressedAudioFormat(&inFormat);
        inputChannels = inFormat.dwSamplesPerFrame;
    }

    UNCOMPRESSEDAUDIOFORMAT outFormat = {};
    if (ppOutputConnections[0]->pFormat) {
        ppOutputConnections[0]->pFormat->GetUncompressedAudioFormat(&outFormat);
        outputChannels = outFormat.dwSamplesPerFrame;
    }

    DebugLogf("Channels: in=%u, out=%u, rate=%.0f", inputChannels, outputChannels, inFormat.fFramesPerSecond);

    HRESULT hr = CBaseAudioProcessingObject::LockForProcess(
        u32NumInputConnections, ppInputConnections,
        u32NumOutputConnections, ppOutputConnections);

    DebugLogf("Base LockForProcess: 0x%08lX", hr);
    firstProcess = true;
    return hr;
}

HRESULT HrtfApo::UnlockForProcess()
{
    DebugLog("UnlockForProcess");
    return CBaseAudioProcessingObject::UnlockForProcess();
}

// ── IAudioProcessingObjectRT ────────────────────────────────────────────

#pragma AVRT_CODE_BEGIN
void HrtfApo::APOProcess(
    UINT32 u32NumInputConnections,
    APO_CONNECTION_PROPERTY** ppInputConnections,
    UINT32 u32NumOutputConnections,
    APO_CONNECTION_PROPERTY** ppOutputConnections)
{
    if (u32NumInputConnections == 0 || u32NumOutputConnections == 0) return;

    float* inputFrames = reinterpret_cast<float*>(ppInputConnections[0]->pBuffer);
    float* outputFrames = reinterpret_cast<float*>(ppOutputConnections[0]->pBuffer);
    UINT32 nFrames = ppInputConnections[0]->u32ValidFrameCount;

    if (firstProcess) {
        firstProcess = false;
        DebugLogf("APOProcess first: frames=%u, in=%u, out=%u, conv=%p, inplace=%d",
                  nFrames, inputChannels, outputChannels, rustConvolver,
                  (inputFrames == outputFrames) ? 1 : 0);
    }

    switch (ppInputConnections[0]->u32BufferFlags)
    {
    case BUFFER_VALID:
    case BUFFER_SILENT:
        if (ppInputConnections[0]->u32BufferFlags == BUFFER_SILENT)
            memset(inputFrames, 0, nFrames * inputChannels * sizeof(float));

        if (inputFrames == outputFrames) {
            // In-place: same buffer, same channel count
            if (inputChannels >= 8 && rustConvolver) {
                hrtf_convolver_process_inplace(rustConvolver, inputFrames, nFrames, inputChannels);
            }
            // else: passthrough — buffer untouched
        } else {
            // Not in-place: input and output may differ in channel count
            if (inputChannels >= 8 && outputChannels >= 2 && rustConvolver) {
                // 8ch input → convolve in-place, then copy to output
                hrtf_convolver_process_inplace(rustConvolver, inputFrames, nFrames, inputChannels);
                memcpy(outputFrames, inputFrames, nFrames * outputChannels * sizeof(float));
            } else {
                // Passthrough: copy input channels, zero-pad remaining output channels
                for (UINT32 f = 0; f < nFrames; f++) {
                    UINT32 copyChans = (inputChannels < outputChannels) ? inputChannels : outputChannels;
                    for (UINT32 c = 0; c < copyChans; c++)
                        outputFrames[f * outputChannels + c] = inputFrames[f * inputChannels + c];
                    for (UINT32 c = copyChans; c < outputChannels; c++)
                        outputFrames[f * outputChannels + c] = 0.0f;
                }
            }
        }

        ppOutputConnections[0]->u32ValidFrameCount = nFrames;

        if (ppInputConnections[0]->u32BufferFlags == BUFFER_SILENT)
            ppOutputConnections[0]->u32BufferFlags = BUFFER_SILENT;
        else
            ppOutputConnections[0]->u32BufferFlags = BUFFER_VALID;
        break;
    }
}
#pragma AVRT_CODE_END
