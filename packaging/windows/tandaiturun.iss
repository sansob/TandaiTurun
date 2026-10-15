; Inno Setup script for the TandaiTurun Windows installer.
; Built in CI with: iscc /DAppVersion=x.y.z /DArch=x64 /DSourceDir=<dir> packaging\windows\tandaiturun.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef Arch
  #define Arch "x64"
#endif
#ifndef SourceDir
  #define SourceDir "..\..\target\release"
#endif

[Setup]
AppId={{3F1B6C2A-8E4D-4F7A-9C55-7D2E1A0B9C31}
AppName=TandaiTurun
AppVersion={#AppVersion}
AppPublisher=TandaiTurun contributors
AppPublisherURL=https://github.com/sansob/TandaiTurun
DefaultDirName={autopf}\TandaiTurun
DefaultGroupName=TandaiTurun
DisableProgramGroupPage=yes
OutputBaseFilename=TandaiTurun-{#AppVersion}-windows-{#Arch}-setup
OutputDir=..\..\dist
SetupIconFile=..\..\assets\icon.ico
UninstallDisplayIcon={app}\tandaiturun.exe
Compression=lzma2
SolidCompression=yes
#if Arch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#endif
PrivilegesRequiredOverridesAllowed=dialog
ChangesAssociations=yes
LicenseFile=..\..\LICENSE-MIT

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "assoc"; Description: "Open .md files with TandaiTurun"; Flags: unchecked

[Files]
Source: "{#SourceDir}\tandaiturun.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\fonts\OFL-*.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion

[Icons]
Name: "{group}\TandaiTurun"; Filename: "{app}\tandaiturun.exe"
Name: "{autodesktop}\TandaiTurun"; Filename: "{app}\tandaiturun.exe"; Tasks: desktopicon

[Registry]
; "Open with" for Markdown files; the default only when the user asked for it.
Root: HKA; Subkey: "Software\Classes\TandaiTurun.Markdown"; ValueType: string; ValueName: ""; ValueData: "Markdown document"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\TandaiTurun.Markdown\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\tandaiturun.exe,0"
Root: HKA; Subkey: "Software\Classes\TandaiTurun.Markdown\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\tandaiturun.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\.md\OpenWithProgids"; ValueType: string; ValueName: "TandaiTurun.Markdown"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.markdown\OpenWithProgids"; ValueType: string; ValueName: "TandaiTurun.Markdown"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.md"; ValueType: string; ValueName: ""; ValueData: "TandaiTurun.Markdown"; Tasks: assoc; Flags: uninsdeletevalue

[Run]
Filename: "{app}\tandaiturun.exe"; Description: "{cm:LaunchProgram,TandaiTurun}"; Flags: nowait postinstall skipifsilent
