#include <windows.h>
#include <aclapi.h>
#include <sddl.h>
#include <cstdio>
#include <new>
#include <audioenginebaseapo.h>
#include "ClassFactory.h"
#include "HrtfApo.h"

// Must use initguid + include guids.h in exactly ONE translation unit
#include <initguid.h>
#include "guids.h"

static HINSTANCE hModule;

static void DebugLog(const char* msg)
{
    FILE* f = nullptr;
    fopen_s(&f, "C:\\ProgramData\\HrtfApo\\debug.log", "a");
    if (f) { fprintf(f, "[HrtfApo DLL] %s\r\n", msg); fclose(f); }
}

static void DebugLogW(const wchar_t* msg)
{
    FILE* f = nullptr;
    fopen_s(&f, "C:\\ProgramData\\HrtfApo\\debug.log", "a");
    if (f) { fwprintf(f, L"[HrtfApo DLL] %s\r\n", msg); fclose(f); }
}

// Note: DllMain is provided by the Rust runtime. We use a helper to capture hModule.
static void CaptureModule()
{
    HMODULE hMod = NULL;
    GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                       (LPCWSTR)CaptureModule, &hMod);
    hModule = hMod;
}

// ── EqualizerAPO-style registry helpers ─────────────────────────────────

static bool EnablePrivilege(LPCWSTR privilegeName)
{
    HANDLE tokenHandle;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &tokenHandle))
        return false;

    LUID luid;
    if (!LookupPrivilegeValueW(NULL, privilegeName, &luid)) {
        CloseHandle(tokenHandle);
        return false;
    }

    TOKEN_PRIVILEGES tp;
    tp.PrivilegeCount = 1;
    tp.Privileges[0].Luid = luid;
    tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

    BOOL ok = AdjustTokenPrivileges(tokenHandle, FALSE, &tp, sizeof(TOKEN_PRIVILEGES), NULL, NULL);
    CloseHandle(tokenHandle);
    return ok && GetLastError() == ERROR_SUCCESS;
}

// Take ownership of a registry key (EqualizerAPO RegistryHelper::takeOwnership)
static LSTATUS TakeOwnership(HKEY root, LPCWSTR subkey)
{
    HKEY hKey;
    LSTATUS st = RegOpenKeyExW(root, subkey, 0, WRITE_OWNER | KEY_WOW64_64KEY, &hKey);
    if (st != ERROR_SUCCESS) return st;

    PSID adminSid = NULL;
    SID_IDENTIFIER_AUTHORITY authority = SECURITY_NT_AUTHORITY;
    AllocateAndInitializeSid(&authority, 2, SECURITY_BUILTIN_DOMAIN_RID, DOMAIN_ALIAS_RID_ADMINS,
                             0, 0, 0, 0, 0, 0, &adminSid);

    PSECURITY_DESCRIPTOR sd = (PSECURITY_DESCRIPTOR)LocalAlloc(LPTR, SECURITY_DESCRIPTOR_MIN_LENGTH);
    InitializeSecurityDescriptor(sd, SECURITY_DESCRIPTOR_REVISION);
    SetSecurityDescriptorOwner(sd, adminSid, FALSE);

    st = RegSetKeySecurity(hKey, OWNER_SECURITY_INFORMATION, sd);

    FreeSid(adminSid);
    LocalFree(sd);
    RegCloseKey(hKey);
    return st;
}

// Grant Administrators full control on a registry key (EqualizerAPO RegistryHelper::makeWritable)
static LSTATUS MakeWritable(HKEY root, LPCWSTR subkey)
{
    HKEY hKey;
    LSTATUS st = RegOpenKeyExW(root, subkey, 0, READ_CONTROL | WRITE_DAC | KEY_WOW64_64KEY, &hKey);
    if (st != ERROR_SUCCESS) return st;

    // Get current DACL
    DWORD descriptorSize = 0;
    RegGetKeySecurity(hKey, DACL_SECURITY_INFORMATION, NULL, &descriptorSize);
    PSECURITY_DESCRIPTOR oldSd = (PSECURITY_DESCRIPTOR)HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, descriptorSize);
    st = RegGetKeySecurity(hKey, DACL_SECURITY_INFORMATION, oldSd, &descriptorSize);
    if (st != ERROR_SUCCESS) { HeapFree(GetProcessHeap(), 0, oldSd); RegCloseKey(hKey); return st; }

    BOOL aclPresent, aclDefaulted;
    PACL oldAcl = NULL;
    GetSecurityDescriptorDacl(oldSd, &aclPresent, &oldAcl, &aclDefaulted);

    // Create Administrators SID
    PSID adminSid = NULL;
    SID_IDENTIFIER_AUTHORITY authority = SECURITY_NT_AUTHORITY;
    AllocateAndInitializeSid(&authority, 2, SECURITY_BUILTIN_DOMAIN_RID, DOMAIN_ALIAS_RID_ADMINS,
                             0, 0, 0, 0, 0, 0, &adminSid);

    // Add KEY_ALL_ACCESS for Administrators
    EXPLICIT_ACCESSW ea = {};
    ea.grfAccessPermissions = KEY_ALL_ACCESS;
    ea.grfAccessMode = SET_ACCESS;
    ea.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
    ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    ea.Trustee.TrusteeType = TRUSTEE_IS_GROUP;
    ea.Trustee.ptstrName = (LPWSTR)adminSid;

    PACL newAcl = NULL;
    SetEntriesInAclW(1, &ea, oldAcl, &newAcl);

    // Apply new DACL
    PSECURITY_DESCRIPTOR sd = (PSECURITY_DESCRIPTOR)LocalAlloc(LPTR, SECURITY_DESCRIPTOR_MIN_LENGTH);
    InitializeSecurityDescriptor(sd, SECURITY_DESCRIPTOR_REVISION);
    SetSecurityDescriptorDacl(sd, TRUE, newAcl, FALSE);

    st = RegSetKeySecurity(hKey, DACL_SECURITY_INFORMATION, sd);

    FreeSid(adminSid);
    LocalFree(newAcl);
    HeapFree(GetProcessHeap(), 0, oldSd);
    LocalFree(sd);
    RegCloseKey(hKey);
    return st;
}

// Write SFX CLSID to FxProperties (EqualizerAPO DeviceAPOInfo::install pattern)
static LSTATUS WriteFxProperties(LPCWSTR endpointSubkey)
{
    wchar_t fxPath[512];
    _snwprintf_s(fxPath, 512, L"%s\\FxProperties", endpointSubkey);

    HKEY hKey;
    LSTATUS st = RegOpenKeyExW(HKEY_LOCAL_MACHINE, fxPath, 0, KEY_SET_VALUE | KEY_QUERY_VALUE | KEY_WOW64_64KEY, &hKey);
    if (st != ERROR_SUCCESS) return st;

    // Write our CLSID to legacy SFX key: {d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5
    const wchar_t* sfxKeyName = L"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5";
    wchar_t clsidStr[] = L"{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}";
    st = RegSetValueExW(hKey, sfxKeyName, 0, REG_SZ, (BYTE*)clsidStr, (DWORD)(wcslen(clsidStr) + 1) * sizeof(wchar_t));

    // Write processing mode if it doesn't exist: {d3993a3f-99c2-4402-b5ec-a92a0367664b},5
    const wchar_t* procModeKeyName = L"{d3993a3f-99c2-4402-b5ec-a92a0367664b},5";
    DWORD dummy;
    if (RegQueryValueExW(hKey, procModeKeyName, NULL, NULL, NULL, &dummy) != ERROR_SUCCESS)
    {
        // Write default processing mode as REG_MULTI_SZ
        const wchar_t defaultMode[] = L"{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}\0";
        st = RegSetValueExW(hKey, procModeKeyName, 0, REG_MULTI_SZ,
                            (BYTE*)defaultMode, sizeof(defaultMode));
    }

    // Delete DisableEnhancements if present
    RegDeleteValueW(hKey, L"{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5");

    RegCloseKey(hKey);
    return st;
}

// ── DLL Entry Points ────────────────────────────────────────────────────

extern "C" HRESULT __stdcall DllCanUnloadNow()
{
    if (HrtfApo::instCount == 0 && ClassFactory::lockCount == 0)
        return S_OK;
    return S_FALSE;
}

extern "C" HRESULT __stdcall DllGetClassObject(REFCLSID rclsid, REFIID riid, void** ppv)
{
    char buf[256];
    snprintf(buf, sizeof(buf), "DllGetClassObject: CLSID=%08lX-%04X-%04X",
             rclsid.Data1, rclsid.Data2, rclsid.Data3);
    DebugLog(buf);

    if (rclsid != CLSID_HrtfApo)
        return CLASS_E_CLASSNOTAVAILABLE;

    ClassFactory* factory = new (std::nothrow) ClassFactory();
    if (!factory) return E_OUTOFMEMORY;

    HRESULT hr = factory->QueryInterface(riid, ppv);
    factory->Release();

    snprintf(buf, sizeof(buf), "DllGetClassObject returning: 0x%08lX", hr);
    DebugLog(buf);
    return hr;
}

extern "C" HRESULT __stdcall DllRegisterServer()
{
    DebugLog("DllRegisterServer called");

    if (!hModule) CaptureModule();
    wchar_t filename[1024];
    GetModuleFileNameW(hModule, filename, sizeof(filename) / sizeof(wchar_t));

    // 1. Register APO via Windows SDK
    HRESULT hr = RegisterAPO(&HrtfApo::regProperties.m_Properties);
    if (FAILED(hr)) {
        char buf[128]; snprintf(buf, sizeof(buf), "RegisterAPO FAILED: 0x%08lX", hr);
        DebugLog(buf);
        UnregisterAPO(CLSID_HrtfApo);
        return hr;
    }

    // 2. Write COM InProcServer32 with actual DLL path
    HKEY hKey;
    wchar_t keyPath[512];
    _snwprintf_s(keyPath, 512, L"SOFTWARE\\Classes\\CLSID\\{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}\\InprocServer32");
    if (RegCreateKeyExW(HKEY_LOCAL_MACHINE, keyPath, 0, NULL, 0, KEY_SET_VALUE | KEY_WOW64_64KEY, NULL, &hKey, NULL) == ERROR_SUCCESS)
    {
        RegSetValueExW(hKey, NULL, 0, REG_SZ, (BYTE*)filename, (DWORD)(wcslen(filename) + 1) * sizeof(wchar_t));
        const wchar_t* threading = L"Both";
        RegSetValueExW(hKey, L"ThreadingModel", 0, REG_SZ, (BYTE*)threading, (DWORD)(wcslen(threading) + 1) * sizeof(wchar_t));
        RegCloseKey(hKey);
    }

    char buf[256];
    snprintf(buf, sizeof(buf), "RegisterAPO + COM OK, DLL: %ls", filename);
    DebugLog(buf);
    return S_OK;
}

extern "C" HRESULT __stdcall DllUnregisterServer()
{
    DebugLog("DllUnregisterServer called");
    UnregisterAPO(CLSID_HrtfApo);

    wchar_t keyPath[512];
    _snwprintf_s(keyPath, 512, L"SOFTWARE\\Classes\\CLSID\\{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}\\InprocServer32");
    RegDeleteKeyW(HKEY_LOCAL_MACHINE, keyPath);
    _snwprintf_s(keyPath, 512, L"SOFTWARE\\Classes\\CLSID\\{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}");
    RegDeleteKeyW(HKEY_LOCAL_MACHINE, keyPath);

    return S_OK;
}

// ── Exported installer function — called from deploy script ─────────────
// Usage: rundll32 hrtf_apo.dll,InstallForEndpoint {endpoint-guid}

extern "C" __declspec(dllexport) void __stdcall InstallForEndpoint(
    HWND hwnd, HINSTANCE hinst, LPSTR lpCmdLine, int nCmdShow)
{
    DebugLog("InstallForEndpoint called");
    DebugLog(lpCmdLine);

    // Convert ANSI cmdline to wide
    wchar_t wideCmdLine[256];
    MultiByteToWideChar(CP_ACP, 0, lpCmdLine, -1, wideCmdLine, 256);

    // lpCmdLine = endpoint GUID, e.g. "{f6654c7e-35f3-4d39-b531-3c0e94f98e55}"
    wchar_t endpointKey[512];
    _snwprintf_s(endpointKey, 512,
        L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\MMDevices\\Audio\\Render\\%s", wideCmdLine);

    // Step 1: Enable SeTakeOwnershipPrivilege
    if (!EnablePrivilege(SE_TAKE_OWNERSHIP_NAME)) {
        DebugLog("Failed to enable SeTakeOwnershipPrivilege");
    } else {
        DebugLog("SeTakeOwnershipPrivilege enabled");
    }

    // Step 2: Take ownership of endpoint key (parent of FxProperties)
    LSTATUS st = TakeOwnership(HKEY_LOCAL_MACHINE, endpointKey);
    char buf[256];
    snprintf(buf, sizeof(buf), "TakeOwnership endpoint: %ld", st);
    DebugLog(buf);

    // Step 3: Make endpoint key writable for Administrators
    st = MakeWritable(HKEY_LOCAL_MACHINE, endpointKey);
    snprintf(buf, sizeof(buf), "MakeWritable endpoint: %ld", st);
    DebugLog(buf);

    // Step 4: Take ownership + make writable for FxProperties subkey
    wchar_t fxKey[512];
    _snwprintf_s(fxKey, 512, L"%s\\FxProperties", endpointKey);

    st = TakeOwnership(HKEY_LOCAL_MACHINE, fxKey);
    snprintf(buf, sizeof(buf), "TakeOwnership FxProperties: %ld", st);
    DebugLog(buf);

    st = MakeWritable(HKEY_LOCAL_MACHINE, fxKey);
    snprintf(buf, sizeof(buf), "MakeWritable FxProperties: %ld", st);
    DebugLog(buf);

    // Step 5: Write our CLSID to FxProperties
    st = WriteFxProperties(endpointKey);
    snprintf(buf, sizeof(buf), "WriteFxProperties: %ld", st);
    DebugLog(buf);

    DebugLog("InstallForEndpoint complete");
}
