//! HRTF APO Installer
//!
//! Registers/unregisters the HRTF APO DLL and associates it with an audio endpoint.
//! Must be run as Administrator.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use windows::core::GUID;
use windows::Win32::System::Registry::*;

/// Must match CLSID_HRTF_APO in hrtf-apo/src/lib.rs
const CLSID_HRTF_APO: GUID = GUID::from_u128(0xa1b2c3d4_e5f6_7890_abcd_ef0123456789);
const APO_FRIENDLY_NAME: &str = "Personalized HRTF Binaural";

/// PKEY_FX_StreamEffectClsid = {D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}, 5
const PKEY_FX_STREAM_EFFECT_CLSID: windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY =
    windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY {
        fmtid: GUID::from_u128(0xD04E05A6_594B_4FB6_A80D_01AF5EED7D1D),
        pid: 5,
    };

#[derive(Parser)]
#[command(name = "hrtf-apo-installer")]
#[command(about = "Install/uninstall the HRTF Audio Processing Object")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install and register the HRTF APO
    Install {
        /// Path to hrtf_apo.dll (defaults to current directory)
        #[arg(long)]
        dll: Option<PathBuf>,
    },
    /// Uninstall and unregister the HRTF APO
    Uninstall,
    /// Show current registration status
    Status,
    /// Associate the APO with an audio render endpoint
    Associate,
    /// Remove APO association from an audio render endpoint
    Disassociate,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { dll } => install(dll),
        Commands::Uninstall => uninstall(),
        Commands::Status => status(),
        Commands::Associate => associate(),
        Commands::Disassociate => disassociate(),
    }
}

fn guid_to_string(guid: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1, guid.data2, guid.data3,
        guid.data4[0], guid.data4[1], guid.data4[2], guid.data4[3],
        guid.data4[4], guid.data4[5], guid.data4[6], guid.data4[7],
    )
}

fn config_dir() -> PathBuf {
    let program_data =
        std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    PathBuf::from(program_data).join("HrtfApo")
}

fn ir_file_path() -> PathBuf {
    config_dir().join("hrtf_irs.bin")
}

fn install(dll_path: Option<PathBuf>) -> Result<()> {
    let dll = dll_path.unwrap_or_else(|| {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_default();
        exe_dir.join("hrtf_apo.dll")
    });

    if !dll.exists() {
        bail!("DLL not found at: {}", dll.display());
    }

    let dll_abs = dll.canonicalize().context("Failed to resolve DLL path")?;
    let dll_str = dll_abs.to_string_lossy();
    let dll_str = dll_str.strip_prefix(r"\\?\").unwrap_or(&dll_str);

    println!("Installing HRTF APO...");
    println!("  DLL: {dll_str}");

    let clsid_str = guid_to_string(&CLSID_HRTF_APO);
    println!("  CLSID: {clsid_str}");

    // 1. Register COM class
    let com_key_path = format!(r"SOFTWARE\Classes\CLSID\{clsid_str}\InProcServer32");
    println!("  Registering COM class...");
    reg_set_string(HKEY_LOCAL_MACHINE, &com_key_path, "", dll_str)?;
    reg_set_string(HKEY_LOCAL_MACHINE, &com_key_path, "ThreadingModel", "Both")?;

    // 2. Register APO
    let apo_key_path = format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\{clsid_str}"
    );
    println!("  Registering APO...");
    reg_set_string(HKEY_LOCAL_MACHINE, &apo_key_path, "FriendlyName", APO_FRIENDLY_NAME)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key_path, "MajorVersion", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key_path, "MinorVersion", 0)?;
    // FRAMESPERSECOND_MUST_MATCH (4) | BITSPERSAMPLE_MUST_MATCH (8) = 0x0C
    // NOT INPLACE (different channel counts), NOT SAMPLESPERFRAME (we change 8ch→2ch)
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key_path, "Flags", 0x0000000C)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key_path, "MinInputConnections", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key_path, "MaxInputConnections", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key_path, "MinOutputConnections", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key_path, "MaxOutputConnections", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key_path, "MaxInstances", 0xFFFFFFFF)?;

    // Create the ProgramData config directory
    let cfg = config_dir();
    std::fs::create_dir_all(&cfg).context("Failed to create config directory")?;
    println!("  Config directory: {}", cfg.display());

    println!();
    println!("APO registered successfully!");
    println!();
    println!("Next steps:");
    println!("  1. Generate personalized IRs using hrtf-synth-gui (Export for APO)");
    println!("  2. Associate APO with your headphone endpoint:");
    println!("       hrtf-apo-installer associate");
    println!("  3. Restart the Windows Audio Service:");
    println!("       net stop audiosrv && net start audiosrv");

    Ok(())
}

fn uninstall() -> Result<()> {
    let clsid_str = guid_to_string(&CLSID_HRTF_APO);
    println!("Uninstalling HRTF APO ({clsid_str})...");

    let com_key = format!(r"SOFTWARE\Classes\CLSID\{clsid_str}");
    let _ = reg_delete_tree(HKEY_LOCAL_MACHINE, &com_key);

    let apo_key = format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\{clsid_str}"
    );
    let _ = reg_delete_tree(HKEY_LOCAL_MACHINE, &apo_key);

    println!("APO unregistered.");
    println!("Note: You may also want to run 'hrtf-apo-installer disassociate' to remove endpoint associations.");
    println!("Then restart: net stop audiosrv && net start audiosrv");

    Ok(())
}

fn status() -> Result<()> {
    let clsid_str = guid_to_string(&CLSID_HRTF_APO);
    println!("HRTF APO Status");
    println!("  CLSID: {clsid_str}");

    let com_key = format!(r"SOFTWARE\Classes\CLSID\{clsid_str}\InProcServer32");
    match reg_get_string(HKEY_LOCAL_MACHINE, &com_key, "") {
        Ok(dll_path) => println!("  COM registered: {dll_path}"),
        Err(_) => println!("  COM: NOT registered"),
    }

    let apo_key = format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\{clsid_str}"
    );
    match reg_get_string(HKEY_LOCAL_MACHINE, &apo_key, "FriendlyName") {
        Ok(name) => println!("  APO registered: {name}"),
        Err(_) => println!("  APO: NOT registered"),
    }

    let ir_path = ir_file_path();
    if ir_path.exists() {
        let size = std::fs::metadata(&ir_path).map(|m| m.len()).unwrap_or(0);
        println!("  IR file: {} ({} bytes)", ir_path.display(), size);
    } else {
        println!("  IR file: NOT FOUND ({})", ir_path.display());
    }

    Ok(())
}

// ── Endpoint association ────────────────────────────────────────────────

fn associate() -> Result<()> {
    use std::io::{self, Write};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;

    // STGM_READ = 0, STGM_READWRITE = 2
    use windows::Win32::System::Com::STGM;
    const STGM_READ: STGM = STGM(0);
    const STGM_READWRITE: STGM = STGM(2);

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

        let collection = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;

        if count == 0 {
            bail!("No active audio render endpoints found");
        }

        println!("Available audio render endpoints:");
        println!();
        for i in 0..count {
            let device = collection.Item(i)?;
            let props = device.OpenPropertyStore(STGM_READ)?;
            let name_pv = props.GetValue(&PKEY_Device_FriendlyName)?;
            let name = propvariant_to_string(&name_pv);
            println!("  [{i}] {name}");
        }

        println!();
        print!("Select endpoint number: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let selection: u32 = input.trim().parse().context("Invalid selection")?;

        if selection >= count {
            bail!("Selection out of range (0-{})", count - 1);
        }

        let device = collection.Item(selection)?;

        // Open property store for writing (requires admin)
        let props = device.OpenPropertyStore(STGM_READWRITE)
            .context("Failed to open property store for writing (are you running as admin?)")?;

        // Write PKEY_FX_StreamEffectClsid = our CLSID string
        let clsid_str = guid_to_string(&CLSID_HRTF_APO);
        let pv = windows_core::PROPVARIANT::from(clsid_str.as_str());
        props.SetValue(&PKEY_FX_STREAM_EFFECT_CLSID, &pv)?;
        props.Commit()?;

        // Read back name for confirmation
        let read_props = device.OpenPropertyStore(STGM_READ)?;
        let name_pv = read_props.GetValue(&PKEY_Device_FriendlyName)?;
        let name = propvariant_to_string(&name_pv);

        println!();
        println!("APO associated with: {name}");
        println!();
        println!("Restart the Windows Audio Service to apply:");
        println!("  net stop audiosrv && net start audiosrv");

        CoUninitialize();
    }

    Ok(())
}

fn disassociate() -> Result<()> {
    use std::io::{self, Write};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;

    use windows::Win32::System::Com::STGM;
    const STGM_READ: STGM = STGM(0);
    const STGM_READWRITE: STGM = STGM(2);

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

        let collection = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;

        if count == 0 {
            bail!("No active audio render endpoints found");
        }

        println!("Available audio render endpoints:");
        println!();
        for i in 0..count {
            let device = collection.Item(i)?;
            let props = device.OpenPropertyStore(STGM_READ)?;
            let name_pv = props.GetValue(&PKEY_Device_FriendlyName)?;
            let name = propvariant_to_string(&name_pv);
            println!("  [{i}] {name}");
        }

        println!();
        print!("Select endpoint to remove APO from: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let selection: u32 = input.trim().parse().context("Invalid selection")?;

        if selection >= count {
            bail!("Selection out of range (0-{})", count - 1);
        }

        let device = collection.Item(selection)?;
        let props = device.OpenPropertyStore(STGM_READWRITE)
            .context("Failed to open property store for writing (are you running as admin?)")?;

        // Remove the stream effect CLSID by setting an empty PROPVARIANT
        let empty = windows_core::PROPVARIANT::default();
        let _ = props.SetValue(&PKEY_FX_STREAM_EFFECT_CLSID, &empty);
        props.Commit()?;

        let read_props = device.OpenPropertyStore(STGM_READ)?;
        let name_pv = read_props.GetValue(&PKEY_Device_FriendlyName)?;
        let name = propvariant_to_string(&name_pv);

        println!();
        println!("APO removed from: {name}");
        println!("Restart: net stop audiosrv && net start audiosrv");

        CoUninitialize();
    }

    Ok(())
}

/// Extract a string from a PROPVARIANT (best-effort)
fn propvariant_to_string(pv: &windows_core::PROPVARIANT) -> String {
    // Try to display it; fall back to "(unknown)"
    format!("{}", pv).trim_matches('"').to_string()
}

// ── Registry helpers ────────────────────────────────────────────────────

fn reg_set_string(root: HKEY, subkey: &str, name: &str, value: &str) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();

        let result = RegCreateKeyW(root, windows::core::PCWSTR(subkey_w.as_ptr()), &mut hkey);
        if result.is_err() {
            bail!("Failed to create registry key: {subkey} (are you running as admin?)");
        }

        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let value_w: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
        let value_bytes: &[u8] =
            std::slice::from_raw_parts(value_w.as_ptr() as *const u8, value_w.len() * 2);

        let result = RegSetValueExW(
            hkey,
            windows::core::PCWSTR(name_w.as_ptr()),
            0,
            REG_SZ,
            Some(value_bytes),
        );
        let _ = RegCloseKey(hkey);

        if result.is_err() {
            bail!("Failed to set registry value: {name}");
        }
    }
    Ok(())
}

fn reg_set_dword(root: HKEY, subkey: &str, name: &str, value: u32) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();

        let result = RegCreateKeyW(root, windows::core::PCWSTR(subkey_w.as_ptr()), &mut hkey);
        if result.is_err() {
            bail!("Failed to create registry key: {subkey}");
        }

        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let value_bytes = value.to_le_bytes();

        let result = RegSetValueExW(
            hkey,
            windows::core::PCWSTR(name_w.as_ptr()),
            0,
            REG_DWORD,
            Some(&value_bytes),
        );
        let _ = RegCloseKey(hkey);

        if result.is_err() {
            bail!("Failed to set registry value: {name}");
        }
    }
    Ok(())
}

fn reg_get_string(root: HKEY, subkey: &str, name: &str) -> Result<String> {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();

        let result = RegOpenKeyExW(
            root,
            windows::core::PCWSTR(subkey_w.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        );
        if result.is_err() {
            bail!("Key not found");
        }

        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut data_type = REG_VALUE_TYPE::default();
        let mut size: u32 = 0;

        let _ = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut data_type),
            None,
            Some(&mut size),
        );

        let mut buffer = vec![0u8; size as usize];
        let result = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut data_type),
            Some(buffer.as_mut_ptr()),
            Some(&mut size),
        );
        let _ = RegCloseKey(hkey);

        if result.is_err() {
            bail!("Value not found");
        }

        let wide: &[u16] =
            std::slice::from_raw_parts(buffer.as_ptr() as *const u16, size as usize / 2);
        let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        Ok(String::from_utf16_lossy(&wide[..len]))
    }
}

fn reg_delete_tree(root: HKEY, subkey: &str) -> Result<()> {
    unsafe {
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let result = RegDeleteTreeW(root, windows::core::PCWSTR(subkey_w.as_ptr()));
        if result.is_err() {
            bail!("Failed to delete registry key: {subkey}");
        }
    }
    Ok(())
}
