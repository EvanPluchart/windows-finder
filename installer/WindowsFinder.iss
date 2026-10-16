; Inno Setup script for Windows Finder. Per-user install: no administrator rights, no UAC prompt.
; Build: iscc /DAppVersion=0.1.0 installer\WindowsFinder.iss   (expects target\release\WindowsFinder.exe)
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6F3A1C52-9B7E-4D0A-8E41-2C5D7B90A1F3}
AppName=Windows Finder
AppVersion={#AppVersion}
AppVerName=Windows Finder {#AppVersion}
AppPublisher=Evan Pluchart
AppPublisherURL=https://github.com/EvanPluchart/windows-finder
AppSupportURL=https://github.com/EvanPluchart/windows-finder/issues
AppUpdatesURL=https://github.com/EvanPluchart/windows-finder/releases
DefaultDirName={localappdata}\Programs\Windows Finder
DefaultGroupName=Windows Finder
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=WindowsFinder-Setup
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\WindowsFinder.exe
LicenseFile=..\LICENSE
Compression=lzma2/ultra
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\WindowsFinder.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Windows Finder"; Filename: "{app}\WindowsFinder.exe"
Name: "{autodesktop}\Windows Finder"; Filename: "{app}\WindowsFinder.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\WindowsFinder.exe"; Description: "{cm:LaunchProgram,Windows Finder}"; Flags: nowait postinstall skipifsilent
