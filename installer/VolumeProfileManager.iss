; VolumeProfileManager インストーラー定義 (Inno Setup)
;
; ビルド前提: 事前に以下のコマンドで Release ビルドを行い、publish\TrayApp に配置しておくこと
;   cargo build --release --locked --target x86_64-pc-windows-msvc
;   mkdir publish\TrayApp
;   copy target\x86_64-pc-windows-msvc\release\VolumeProfileManager_TrayApp.exe publish\TrayApp\VolumeProfileManager.TrayApp.exe
;   copy assets\app.ico publish\TrayApp\app.ico
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
CloseApplications=yes
CloseApplicationsFilter=VolumeProfileManager.TrayApp.exe,VolumeProfileManager.exe

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "startupicon"; Description: "Windowsログオン時に自動起動する"; GroupDescription: "追加のオプション:"; Flags: checkedonce

[Files]
Source: "{#PublishDir}\VolumeProfileManager.TrayApp.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PublishDir}\app.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{#MyAppName} をアンインストール"; Filename: "{uninstallexe}"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#MyAppName}"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: startupicon; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{#MyAppName} を起動する"; Flags: nowait postinstall skipifsilent

[Code]
// インストール前に古い .NET 版の残留ファイルをクリーンアップする
procedure CurStepChanged(CurStep: TSetupStep);
var
  FindRec: TFindRec;
  AppPath: string;
begin
  if CurStep = ssInstall then
  begin
    AppPath := ExpandConstant('{app}');
    if DirExists(AppPath) then
    begin
      // 古い .dll, .pdb, .runtimeconfig.json, .deps.json を削除
      if FindFirst(AppPath + '\*.dll', FindRec) then
      begin
        try
          repeat
            DeleteFile(AppPath + '\' + FindRec.Name);
          until not FindNext(FindRec);
        finally
          FindClose(FindRec);
        end;
      end;
      if FindFirst(AppPath + '\*.pdb', FindRec) then
      begin
        try
          repeat
            DeleteFile(AppPath + '\' + FindRec.Name);
          until not FindNext(FindRec);
        finally
          FindClose(FindRec);
        end;
      end;
      if FindFirst(AppPath + '\*.json', FindRec) then
      begin
        try
          repeat
            DeleteFile(AppPath + '\' + FindRec.Name);
          until not FindNext(FindRec);
        finally
          FindClose(FindRec);
        end;
      end;
    end;
  end;
end;
