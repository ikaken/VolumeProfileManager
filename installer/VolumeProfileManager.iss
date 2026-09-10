; VolumeProfileManager インストーラー定義 (Inno Setup)
;
; ビルド前提: 事前に以下のコマンドで Self-contained 発行を行っておくこと
;   dotnet publish ..\src\VolumeProfileManager.TrayApp\VolumeProfileManager.TrayApp.csproj ^
;     -c Release -r win-x64 --self-contained true -o ..\publish\TrayApp
;
; ビルド方法（Inno Setup Compiler がインストールされていること）:
;   ISCC.exe VolumeProfileManager.iss

#define MyAppName "VolumeProfileManager"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "VolumeProfileManager Project"
#define MyAppExeName "VolumeProfileManager.TrayApp.exe"
#define PublishDir "..\publish\TrayApp"

[Setup]
AppId={{8F2B6C6E-3D2E-4C7A-9A8B-1E5D6F7A9C10}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
; 管理者権限不要のユーザーローカルインストール（%LOCALAPPDATA%\Programs配下）
DefaultDirName={userpf}\{#MyAppName}
DefaultGroupName={#MyAppName}
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=VolumeProfileManagerSetup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#MyAppExeName}
SetupIconFile=..\assets\app.ico
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "startupicon"; Description: "Windowsログオン時に自動起動する"; GroupDescription: "追加のオプション:"; Flags: checkedonce

[Files]
Source: "{#PublishDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{#MyAppName} をアンインストール"; Filename: "{uninstallexe}"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#MyAppName}"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: startupicon; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{#MyAppName} を起動する"; Flags: nowait postinstall skipifsilent
