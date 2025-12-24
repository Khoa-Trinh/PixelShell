; Script generated for Pixel Shell
#define MyAppName "Pixel Shell"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "ShineeKun"
#define MyAppURL "https://github.com/Khoa-Trinh/PixelShell"
#define MyAppExeName "ps-cli.exe"
#define MyGuiExeName "ps-gui.exe"
#define MyInstallerName "pixel-shell-setup"

[Setup]
AppId={{88306388-509C-4E15-95DD-59BDD4ACC8B5}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}

; Install to User AppData (No Admin Required)
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
PrivilegesRequired=lowest
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
Name: "envPath"; Description: "Add to System PATH (Required for CLI usage)"; GroupDescription: "Configuration:"; Flags: checkedonce
Name: "installDeps"; Description: "Download FFmpeg & yt-dlp (Required for video processing)"; GroupDescription: "Dependencies:"; Flags: checkedonce
Name: "desktopIcon"; Description: "Create a Desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Files]
; --- ADDED ps-gui.exe HERE ---
Source: "target\release\ps-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\ps-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\ps-runner.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Dirs]
Name: "{app}\assets"
Name: "{app}\dist"

[Icons]
; Shortcut for the GUI (Main Entry Point)
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyGuiExeName}"
; Shortcut for the CLI
Name: "{group}\{#MyAppName} CLI"; Filename: "{app}\{#MyAppExeName}"
; Uninstall Shortcut
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
; Desktop Shortcut (GUI)
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyGuiExeName}"; Tasks: desktopIcon

[Registry]
; Add {app} to PATH. Since we put ffmpeg/yt-dlp in {app}, this covers everything.
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: envPath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
; Option to launch the GUI after installation
Filename: "{app}\{#MyGuiExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent

[Code]
// --- PATH CHECKER ---
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

// --- DOWNLOADER LOGIC ---
procedure CurStepChanged(CurStep: TSetupStep);
var
  ProgressPage: TOutputProgressWizardPage;
  ResultCode: Integer;
  AppDir: String;
  UrlYtDlp: String;
  UrlFFmpeg: String;
  PsCmd: String;
begin
  // FIX: Used 'WizardIsTaskSelected' instead of 'IsTaskSelected'
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('installDeps') then
  begin
    UrlYtDlp := 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe';
    UrlFFmpeg := 'https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip';
    AppDir := ExpandConstant('{app}');

    ProgressPage := CreateOutputProgressPage('Downloading Dependencies', 'Please wait while we fetch the latest FFmpeg and yt-dlp...');
    ProgressPage.Show;

    try
      ProgressPage.SetText('Downloading yt-dlp...', '');
      ProgressPage.SetProgress(10, 100);

      // 1. Download yt-dlp
      PsCmd := '-Command "Invoke-WebRequest -Uri ''' + UrlYtDlp + ''' -OutFile ''' + AppDir + '\yt-dlp.exe''"';
      if not Exec('powershell.exe', PsCmd, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
      begin
        MsgBox('Failed to download yt-dlp.', mbError, MB_OK);
      end;

      ProgressPage.SetText('Downloading and Extracting FFmpeg...', '');
      ProgressPage.SetProgress(40, 100);

      // 2. Download FFmpeg Zip -> Extract -> Move Binaries -> Cleanup
      PsCmd := '-Command "try { ' +
               'Invoke-WebRequest -Uri ''' + UrlFFmpeg + ''' -OutFile ''' + AppDir + '\ffmpeg.zip''; ' +
               'Expand-Archive -Path ''' + AppDir + '\ffmpeg.zip'' -DestinationPath ''' + AppDir + '\ffmpeg_tmp'' -Force; ' +
               'Move-Item -Path ''' + AppDir + '\ffmpeg_tmp\*\bin\ffmpeg.exe'' -Destination ''' + AppDir + ''' -Force; ' +
               'Move-Item -Path ''' + AppDir + '\ffmpeg_tmp\*\bin\ffprobe.exe'' -Destination ''' + AppDir + ''' -Force; ' +
               'Remove-Item -Path ''' + AppDir + '\ffmpeg_tmp'' -Recurse -Force; ' +
               'Remove-Item -Path ''' + AppDir + '\ffmpeg.zip'' -Force; ' +
               '} catch { exit 1 }"';

      if Exec('powershell.exe', PsCmd, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
      begin
         if ResultCode <> 0 then
           MsgBox('Error downloading or extracting FFmpeg.', mbError, MB_OK);
      end
      else begin
         MsgBox('Failed to launch PowerShell.', mbError, MB_OK);
      end;

      ProgressPage.SetProgress(100, 100);
    finally
      ProgressPage.Hide;
    end;
  end;
end;
