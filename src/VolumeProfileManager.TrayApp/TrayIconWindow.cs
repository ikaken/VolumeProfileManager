using System;
using System.Runtime.InteropServices;
using Serilog;
using static VolumeProfileManager.TrayApp.NativeMethods;

namespace VolumeProfileManager.TrayApp;

/// <summary>
/// Win32 API を P/Invoke で直接呼び出し、WinForms/WPF に依存せずタスクトレイアイコンを実装する。
/// </summary>
public sealed class TrayIconWindow : IDisposable
{
    public const int CmdStatus = 1001;
    public const int CmdToggleStartup = 1002;
    public const int CmdExit = 1003;
    public const int CmdUpdateProfile = 1004;

    private const string ClassName = "VolumeProfileManagerTrayWindowClass";
    private const int TrayIconId = 1;

    private readonly WndProcDelegate _wndProcDelegate;
    private readonly IntPtr _hWnd;
    private readonly IntPtr _hIcon;
    private bool _disposed;
    private static readonly ILogger Logger = Log.ForContext<TrayIconWindow>();

    public event Action<int>? MenuCommandSelected;

    public TrayIconWindow()
    {
        _wndProcDelegate = WndProc;
        _hWnd = CreateNativeWindow();
        _hIcon = LoadIcon(IntPtr.Zero, (IntPtr)IDI_APPLICATION);
        AddTrayIcon();
    }

    private IntPtr CreateNativeWindow()
    {
        var hInstance = GetModuleHandle(null);

        var wcex = new WNDCLASSEX
        {
            cbSize = Marshal.SizeOf<WNDCLASSEX>(),
            style = 0,
            lpfnWndProc = _wndProcDelegate,
            cbClsExtra = 0,
            cbWndExtra = 0,
            hInstance = hInstance,
            hIcon = IntPtr.Zero,
            hCursor = IntPtr.Zero,
            hbrBackground = IntPtr.Zero,
            lpszMenuName = null,
            lpszClassName = ClassName,
            hIconSm = IntPtr.Zero,
        };

        RegisterClassEx(ref wcex);

        return CreateWindowEx(
            0, ClassName, "VolumeProfileManager", 0,
            0, 0, 0, 0,
            IntPtr.Zero, IntPtr.Zero, hInstance, IntPtr.Zero);
    }

    private void AddTrayIcon()
    {
        var data = new NOTIFYICONDATA
        {
            cbSize = Marshal.SizeOf<NOTIFYICONDATA>(),
            hWnd = _hWnd,
            uID = TrayIconId,
            uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage = (int)WM_TRAYICON,
            hIcon = _hIcon,
            szTip = "VolumeProfileManager",
            szInfo = string.Empty,
            szInfoTitle = string.Empty,
        };

        var added = Shell_NotifyIcon(NIM_ADD, ref data);
        Logger.Information("Shell_NotifyIcon(NIM_ADD) result={Result}, hWnd={HWnd}, hIcon={HIcon}", added, _hWnd, _hIcon);
        if (!added)
        {
            Logger.Warning("Shell_NotifyIcon(NIM_ADD) failed. Win32Error={Error}", Marshal.GetLastWin32Error());
        }
    }

    public void ShowBalloon(string title, string message)
    {
        var data = new NOTIFYICONDATA
        {
            cbSize = Marshal.SizeOf<NOTIFYICONDATA>(),
            hWnd = _hWnd,
            uID = TrayIconId,
            uFlags = NIF_INFO,
            szTip = "VolumeProfileManager",
            szInfo = message,
            szInfoTitle = title,
            dwInfoFlags = NIIF_INFO,
        };

        var modified = Shell_NotifyIcon(NIM_MODIFY, ref data);
        Logger.Information("Shell_NotifyIcon(NIM_MODIFY/balloon) result={Result}, title={Title}, message={Message}", modified, title, message);
        if (!modified)
        {
            Logger.Warning("Shell_NotifyIcon(NIM_MODIFY) failed. Win32Error={Error}", Marshal.GetLastWin32Error());
        }
    }

    private IntPtr WndProc(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam)
    {
        if (msg == WM_TRAYICON)
        {
            var mouseMsg = (int)lParam & 0xFFFF;
            if (mouseMsg == WM_LBUTTONUP || mouseMsg == WM_RBUTTONUP)
            {
                ShowContextMenu();
            }
            return IntPtr.Zero;
        }

        if (msg == WM_DESTROY)
        {
            PostQuitMessage(0);
            return IntPtr.Zero;
        }

        return DefWindowProc(hWnd, msg, wParam, lParam);
    }

    private void ShowContextMenu()
    {
        var hMenu = CreatePopupMenu();
        AppendMenu(hMenu, MF_STRING, (IntPtr)CmdStatus, "ステータス表示");
        AppendMenu(hMenu, MF_STRING, (IntPtr)CmdUpdateProfile, "プロファイルを更新（現在の音量を保存）");
        AppendMenu(hMenu, MF_STRING, (IntPtr)CmdToggleStartup, "スタートアップ登録/解除");
        AppendMenu(hMenu, MF_STRING, (IntPtr)CmdExit, "終了");

        GetCursorPos(out var pt);
        SetForegroundWindow(_hWnd);

        var cmd = TrackPopupMenuEx(hMenu, TPM_RIGHTBUTTON | TPM_RETURNCMD, pt.X, pt.Y, _hWnd, IntPtr.Zero);

        DestroyMenu(hMenu);

        if (cmd != 0)
        {
            MenuCommandSelected?.Invoke((int)cmd);
        }
    }

    public void RequestExit()
    {
        DestroyWindow(_hWnd);
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        var data = new NOTIFYICONDATA
        {
            cbSize = Marshal.SizeOf<NOTIFYICONDATA>(),
            hWnd = _hWnd,
            uID = TrayIconId,
        };
        Shell_NotifyIcon(NIM_DELETE, ref data);
        _disposed = true;
    }
}
