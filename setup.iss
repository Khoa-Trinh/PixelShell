; Script generated for Pixel Shell
#define MyAppName "Pixel Shell"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "ShineeKun"
#define MyAppURL "https://github.com/Khoa-Trinh/PixelShell"
#define MyAppExeName "ps-cli.exe"
#define MyInstallerName "pixel-shell-setup"

[Setup]
AppId={{88306388-509C-4E15-95DD-59BDD4ACC8B5}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}

; --- FIX 1: Use 'autopf' with 'lowest' privileges ---
; This installs to C:\Users\<Name>\AppData\Local\Programs\Pixel Shell
; No Admin rights required, and valid for HKCU registry changes.
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
PrivilegesRequired=lowest

; --- FIX 2: Use modern architecture identifier ---
ArchitecturesInstallIn64BitMode=x64compatible

OutputDir=.
OutputBaseFilename={#MyInstallerName}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "envPath"; Description: "Add to System PATH (Required for CLI usage)"; GroupDescription: "Additional icons:"; Flags: checkedonce

[Files]
Source: "target\release\ps-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\ps-runner.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Dirs]
Name: "{app}\assets"
Name: "{app}\dist"

[Icons]
Name: "{group}\{#MyAppName} CLI"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

[Registry]
; This is now safe because we are running as the User (lowest privileges)
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: envPath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;
