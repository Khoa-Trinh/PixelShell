use std::env;
use std::path::Path;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let mut res = winres::WindowsResource::new();

    let icon_path = "../../pixel-shell.ico";

    if !Path::new(icon_path).exists() {
        println!("cargo:warning=⚠️ Icon not found at: {}", icon_path);
    } else {
        res.set_icon(icon_path);
    }

    res.set(
        "FileDescription",
        "Pixel Shell High-Performance Overlay Engine",
    );
    res.set("ProductName", "Pixel Shell");
    res.set("CompanyName", "ShineeKun");
    res.set("FileVersion", "1.0.0.0");
    res.set("ProductVersion", "1.0.0.0");
    res.set(
        "LegalCopyright",
        "Copyright © 2025 ShineeKun. All Rights Reserved.",
    );
    res.set("OriginalFilename", "ps-runner.exe");

    res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="PixelShell.Runner" version="1.0.0.0" processorArchitecture="*" />

  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>

  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>

  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
"#);

    if let Err(e) = res.compile() {
        panic!("Resource Compile Error: {}", e);
    }
}
