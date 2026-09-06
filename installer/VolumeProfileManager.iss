; VolumeProfileManager インストーラー定義 (Inno Setup)
;
; ビルド前提:
;   cargo build --release --locked --target x86_64-pc-windows-msvc
;   上記で生成された target\x86_64-pc-windows-msvc\release\VolumeProfileManager_TrayApp.exe と
;   assets\app.ico を publish\TrayApp へ配置しておくこと
;
; ビルド方法（Inno Setup Compiler がインストールされていること）:
;   ISCC.exe /DMyAppVersion=2.0.0-beta VolumeProfileManager.iss

#ifndef MyAppVersion
#define MyAppVersion "2.0.0-beta"
#endif

#define MyAppName "VolumeProfileManager"
#define MyAppPublisher "VolumeProfileManager Project"
#define MyAppExeName "VolumeProfileManager.TrayApp.exe"
#define PublishDir "..\publish\TrayApp"

[Setup]
AppId={{8F2B6C6E-3D2E-4C7A-9A8B-1E5D6F7A9C10}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
; 管理者権限不要のユーザーローカルインストール（%LOCALAPPDATA%\Programs 配下）
DefaultDirName={userpf}\{#MyAppName}
DefaultGroupName={#MyAppName}
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=VolumeProfileManagerSetup
Compression=zip
SolidCompression=no
WizardStyle=modern
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#MyAppExeName}
SetupIconFile=..\assets\app.ico
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
; 置換インストール時に既存プロセスのファイルロックを回避するための標準機能
CloseApplications=yes
CloseApplicationsFilter=VolumeProfileManager.TrayApp.exe

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Files]
Source: "{#PublishDir}\VolumeProfileManager.TrayApp.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PublishDir}\app.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{#MyAppName} をアンインストール"; Filename: "{uninstallexe}"
