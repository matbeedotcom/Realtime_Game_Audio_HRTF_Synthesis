#include "ClassFactory.h"
#include "HrtfApo.h"
#include <new>

long ClassFactory::lockCount = 0;

ClassFactory::ClassFactory() : refCount(1) {}

HRESULT ClassFactory::QueryInterface(REFIID riid, void** ppv)
{
    if (!ppv) return E_POINTER;
    *ppv = nullptr;

    if (riid == __uuidof(IUnknown) || riid == __uuidof(IClassFactory)) {
        *ppv = static_cast<IClassFactory*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

ULONG ClassFactory::AddRef()  { return InterlockedIncrement(&refCount); }

ULONG ClassFactory::Release()
{
    long count = InterlockedDecrement(&refCount);
    if (count == 0) { delete this; return 0; }
    return count;
}

HRESULT ClassFactory::CreateInstance(IUnknown* pUnkOuter, REFIID riid, void** ppv)
{
    if (!ppv) return E_POINTER;
    *ppv = nullptr;

    // Support aggregation (matches EqualizerAPO pattern)
    if (pUnkOuter != NULL && riid != __uuidof(IUnknown))
        return CLASS_E_NOAGGREGATION;

    HrtfApo* apo = new (std::nothrow) HrtfApo(pUnkOuter);
    if (!apo) return E_OUTOFMEMORY;

    HRESULT hr = apo->NonDelegatingQueryInterface(riid, ppv);
    apo->NonDelegatingRelease();
    return hr;
}

HRESULT ClassFactory::LockServer(BOOL fLock)
{
    if (fLock) InterlockedIncrement(&lockCount);
    else       InterlockedDecrement(&lockCount);
    return S_OK;
}
