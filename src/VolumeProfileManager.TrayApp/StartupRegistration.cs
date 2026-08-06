using Microsoft.Win32;

namespace VolumeProfileManager.TrayApp;

/// <summary>
/// Windows起動時の自動起動をレジストリ(HKCU\...\Run)で管理する。
/// 将来インストーラーから同じロジックを再利用できるよう、独立したクラスとして切り出している。
/// </summary>
public static class StartupRegistration
{
    private const string RunKeyPath = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string ValueName = "VolumeProfileManager";

    public static bool IsRegistered()
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKeyPath, writable: false);
        return key?.GetValue(ValueName) != null;
    }

    public static void Register(string executablePath)
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKeyPath, writable: true)
            ?? Registry.CurrentUser.CreateSubKey(RunKeyPath);
        key.SetValue(ValueName, $"\"{executablePath}\"");
    }

    public static void Unregister()
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKeyPath, writable: true);
        key?.DeleteValue(ValueName, throwOnMissingValue: false);
    }

    /// <returns>トグル後に登録状態であれば true、解除状態であれば false</returns>
    public static bool Toggle(string executablePath)
    {
        if (IsRegistered())
        {
            Unregister();
            return false;
        }

        Register(executablePath);
        return true;
    }
}
