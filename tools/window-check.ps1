param(
  [ValidateSet('inspect','move','restore','capture')][string]$Action = 'inspect',
  [int]$X = 120, [int]$Y = 100,
  [ValidateRange(420,20000)][int]$Width = 1600,
  [ValidateRange(400,20000)][int]$Height = 1100
)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PinmeterWindowCheck {
  [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr window, int command);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr window, uint attribute, out int value, uint size);
}
'@
[PinmeterWindowCheck]::SetThreadDpiAwarenessContext([IntPtr]::new(-4)) | Out-Null
$projectRoot = Split-Path $PSScriptRoot -Parent
$process = Get-Process pinmeter-host -ErrorAction Stop | Where-Object { $_.Path.StartsWith("$projectRoot\src\backend\target\", [StringComparison]::OrdinalIgnoreCase) } | Select-Object -First 1
if (!$process) { throw 'No project Pinmeter window found' }
$handle = $process.MainWindowHandle
if ($Action -eq 'move') { [PinmeterWindowCheck]::SetWindowPos($handle, [IntPtr]::Zero, $X, $Y, $Width, $Height, 4) | Out-Null }
if ($Action -eq 'restore') { [PinmeterWindowCheck]::ShowWindowAsync($handle, 9) | Out-Null }
$rect = New-Object PinmeterWindowCheck+Rect
[PinmeterWindowCheck]::GetWindowRect($handle, [ref]$rect) | Out-Null
if ($Action -eq 'capture') {
    Add-Type -AssemblyName System.Drawing
    $bitmap = [Drawing.Bitmap]::new($rect.Right-$rect.Left, $rect.Bottom-$rect.Top)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $dc = $graphics.GetHdc()
    try { [PinmeterWindowCheck]::PrintWindow($handle,$dc,2) | Out-Null } finally { $graphics.ReleaseHdc($dc) }
    $outputDir = "$projectRoot/src/backend/target/measurements"
    New-Item -ItemType Directory -Force $outputDir | Out-Null
    $bitmap.Save("$outputDir/release-window.png", [Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose(); $bitmap.Dispose()
}
$darkCaption=0
$themeResult=[PinmeterWindowCheck]::DwmGetWindowAttribute($handle,20,[ref]$darkCaption,4)
[pscustomobject]@{ ProcessId=$process.Id; Left=$rect.Left; Top=$rect.Top; Width=$rect.Right-$rect.Left; Height=$rect.Bottom-$rect.Top; Dpi=[PinmeterWindowCheck]::GetDpiForWindow($handle); DarkCaption=if($themeResult -eq 0){$darkCaption -ne 0}else{$null}; ThemeResult=$themeResult } | ConvertTo-Json
