fn main() {
    // Find Windows SDK paths
    let sdk_include = "C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0";
    let sdk_lib = "C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0/um/x64";
    let msvc_include = "C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/include";
    let atl_include = "C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/atlmfc/include";

    // Compile C++ APO code
    cc::Build::new()
        .cpp(true)
        .file("cpp/DllMain.cpp")
        .file("cpp/ClassFactory.cpp")
        .file("cpp/HrtfApo.cpp")
        .include(msvc_include)
        .include(atl_include)
        .include(format!("{}/um", sdk_include))
        .include(format!("{}/shared", sdk_include))
        .include(format!("{}/ucrt", sdk_include))
        .include("cpp")
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .flag("/EHsc")  // C++ exceptions
        .compile("hrtf_apo_cpp");

    // Link against Windows SDK APO library and ATL
    let atl_lib = "C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/atlmfc/lib/x64";
    println!("cargo:rustc-link-search=native={}", sdk_lib);
    println!("cargo:rustc-link-search=native={}", atl_lib);
    println!("cargo:rustc-link-lib=static=audiobaseprocessingobject");
    println!("cargo:rustc-link-lib=static=audiomediatypecrt");
    println!("cargo:rustc-link-lib=dylib=ole32");
    println!("cargo:rustc-link-lib=dylib=oleaut32");
    println!("cargo:rustc-link-lib=dylib=uuid");
    println!("cargo:rustc-link-lib=dylib=avrt");

    // Needed for RegisterAPO (_vsnwprintf from legacy CRT)
    let ucrt_lib = "C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0/ucrt/x64";
    println!("cargo:rustc-link-search=native={}", ucrt_lib);
    println!("cargo:rustc-link-lib=static=legacy_stdio_definitions");

    // Export the DLL entry points via .def file
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let def_path = std::path::Path::new(&manifest_dir).join("cpp").join("hrtf_apo.def");
    println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def_path.display());

    println!("cargo:rerun-if-changed=cpp/");
}
