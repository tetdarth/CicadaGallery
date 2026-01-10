; CicadaGallery Installer Script for Inno Setup
; Inno Setup: https://jrsoftware.org/isinfo.php

#define MyAppName "CicadaGallery"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "CicadaGallery"
#define MyAppURL "https://github.com/cicadagallery"
#define MyAppExeName "cicada_gallery.exe"

[Setup]
; NOTE: The value of AppId uniquely identifies this application.
AppId={{B8A3E8F2-5C6D-4E7A-9B8C-1D2E3F4A5B6C}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
; Output settings
OutputDir=..\dist
OutputBaseFilename=CicadaGallery-{#MyAppVersion}-Setup
SetupIconFile=..\image\cicadaGallery.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
; Require admin for Program Files installation
PrivilegesRequired=admin
; Uninstaller settings
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "quicklaunchicon"; Description: "{cm:CreateQuickLaunchIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked; OnlyBelowVersion: 6.1; Check: not IsAdminInstallMode

[Files]
; Main application
Source: "..\target\release\cicada_gallery.exe"; DestDir: "{app}"; Flags: ignoreversion

; Application icon (for runtime loading)
Source: "..\image\cicadaGallery.ico"; DestDir: "{app}\image"; Flags: ignoreversion

; Documentation
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD_PARTY_LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion

; MPV - Essential components only
Source: "..\mpv\mpv.exe"; DestDir: "{app}\mpv"; Flags: ignoreversion
Source: "..\mpv\mpv.com"; DestDir: "{app}\mpv"; Flags: ignoreversion
Source: "..\mpv\d3dcompiler_43.dll"; DestDir: "{app}\mpv"; Flags: ignoreversion
Source: "..\mpv\glsl_shaders\*"; DestDir: "{app}\mpv\glsl_shaders"; Flags: ignoreversion recursesubdirs createallsubdirs

; FFmpeg - Essential components only
Source: "..\ffmpeg\bin\ffmpeg.exe"; DestDir: "{app}\ffmpeg\bin"; Flags: ignoreversion
Source: "..\ffmpeg\bin\ffprobe.exe"; DestDir: "{app}\ffmpeg\bin"; Flags: ignoreversion
Source: "..\ffmpeg\LICENSE.txt"; DestDir: "{app}\ffmpeg"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Clean up cache directory on uninstall (optional, user can keep data)
Type: filesandordirs; Name: "{app}\cache"
