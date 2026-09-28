param([switch]$Embed, [string]$OutputPath, [string]$CapturePath, [int]$CaptureAbove = 0, [switch]$ExerciseDisplay, [switch]$DisplayOnly)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
if ($OutputPath) { Start-Transcript -Path $OutputPath -Force | Out-Null }
# Run in a separate PowerShell process: DPI changes must not touch the app.
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, WindowsBase
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class TaskbarProbe {
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; public override string ToString(){return Left+","+Top+","+Right+","+Bottom;} }
 [StructLayout(LayoutKind.Sequential)] public struct Point {public int X,Y;}
 public delegate bool EnumProc(IntPtr h, IntPtr p);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c,string t);
 [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h,EnumProc f,IntPtr p);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder s,int n);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr GetWindowDpiAwarenessContext(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr GetThreadDpiAwarenessContext();
 [DllImport("user32.dll")] public static extern IntPtr GetDesktopWindow();
 [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
 [DllImport("kernel32.dll")] public static extern void SetLastError(uint e);
 [DllImport("user32.dll")] public static extern int GetAwarenessFromDpiAwarenessContext(IntPtr c);
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
 [DllImport("user32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern IntPtr CreateWindowEx(int ex,string c,string t,int style,int x,int y,int w,int h,IntPtr p,IntPtr menu,IntPtr instance,IntPtr param);
 [DllImport("user32.dll",SetLastError=true)] public static extern IntPtr SetParent(IntPtr h,IntPtr p);
 [DllImport("user32.dll")] public static extern bool DestroyWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool GetCursorPos(out Point p);
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 public static IntPtr OwnDisplay(IntPtr bar){IntPtr result=IntPtr.Zero;EnumChildWindows(bar,(h,p)=>{var s=new StringBuilder(256);GetClassName(h,s,256);if(s.ToString()=="PinmeterTaskbarDisplay"){uint pid;GetWindowThreadProcessId(h,out pid);var name=System.Diagnostics.Process.GetProcessById((int)pid).MainModule.FileName;if(name.EndsWith("taskbar-probe.exe",StringComparison.OrdinalIgnoreCase))result=h;}return true;},IntPtr.Zero);return result;}
 public static string Info(IntPtr h){ var s=new StringBuilder(256); GetClassName(h,s,256); Rect r;GetWindowRect(h,out r);return h+" "+s+" rect="+r+" dpi="+GetDpiForWindow(h)+" awareness="+GetAwarenessFromDpiAwarenessContext(GetWindowDpiAwarenessContext(h)); }
 public static void Inspect(IntPtr h){Console.WriteLine(Info(h));EnumChildWindows(h,(c,p)=>{Console.WriteLine(Info(c));return true;},IntPtr.Zero);}
}
'@
$bar = [TaskbarProbe]::FindWindow('Shell_TrayWnd',$null)
if ($bar -eq [IntPtr]::Zero) { throw 'Explorer taskbar unavailable' }
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
Write-Output ('elevated=' + $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
[TaskbarProbe]::SetThreadDpiAwarenessContext([IntPtr](-4)) | Out-Null
[TaskbarProbe]::Inspect($bar)
$root = [System.Windows.Automation.AutomationElement]::FromHandle($bar)
$nodes = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
foreach ($node in $nodes) {
    $info = $node.Current
    Write-Output ('UIA ' + $info.ControlType.ProgrammaticName + ' ' + $info.ClassName + ' ' + $info.AutomationId + ' rect=' + $info.BoundingRectangle + ' offscreen=' + $info.IsOffscreen)
}
if ($Embed) {
    # Invisible 1px window: tests cross-process parenting without covering buttons.
    $child = [TaskbarProbe]::CreateWindowEx(0,'STATIC','Pinmeter taskbar probe',0x40000000,0,0,1,1,[TaskbarProbe]::GetDesktopWindow(),[IntPtr]::Zero,[IntPtr]::Zero,[IntPtr]::Zero)
    if ($child -eq [IntPtr]::Zero) { throw 'CreateWindowEx failed' }
    try {
        $before = [TaskbarProbe]::GetThreadDpiAwarenessContext()
        [TaskbarProbe]::SetLastError(0)
        $previousParent = [TaskbarProbe]::SetParent($child,$bar)
        $errorCode = if ($previousParent -eq [IntPtr]::Zero) { [Runtime.InteropServices.Marshal]::GetLastWin32Error() } else { 0 }
        Write-Output ('embed error=' + $errorCode + ' before=' + $before + ' after=' + [TaskbarProbe]::GetThreadDpiAwarenessContext())
        [TaskbarProbe]::Info($child)
        if ([TaskbarProbe]::GetParent($child) -ne $bar) { throw 'Parent did not change to Explorer' }
    } finally { [TaskbarProbe]::DestroyWindow($child) | Out-Null }
}
if ($ExerciseDisplay) {
    $display = [TaskbarProbe]::OwnDisplay($bar)
    if ($display -eq [IntPtr]::Zero) { throw 'Run the native taskbar-probe example before exercising display messages' }
    $previousCursor = New-Object TaskbarProbe+Point
    [TaskbarProbe]::GetCursorPos([ref]$previousCursor) | Out-Null
    $displayRect = New-Object TaskbarProbe+Rect
    [TaskbarProbe]::GetWindowRect($display,[ref]$displayRect) | Out-Null
    [TaskbarProbe]::SetCursorPos(($displayRect.Left+40),($displayRect.Top+10)) | Out-Null
    # Only the standalone example receives simulated input; actions print to its console.
    [TaskbarProbe]::SendMessage($display,0x0200,[IntPtr]::Zero,[IntPtr]((10 -shl 16) -bor 40)) | Out-Null
    [TaskbarProbe]::SendMessage($display,0x0202,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
    [TaskbarProbe]::SendMessage($display,0x0101,[IntPtr]13,[IntPtr]::Zero) | Out-Null
    Start-Sleep -Milliseconds 800
}
if ($CapturePath) {
    Add-Type -AssemblyName System.Drawing
    $rect = New-Object TaskbarProbe+Rect
    [TaskbarProbe]::GetWindowRect($bar,[ref]$rect) | Out-Null
    if ($DisplayOnly) {
        $display = [TaskbarProbe]::OwnDisplay($bar)
        if ($display -eq [IntPtr]::Zero) { throw 'Native probe display unavailable for capture' }
        [TaskbarProbe]::GetWindowRect($display,[ref]$rect) | Out-Null
        $rect.Left = $rect.Right - [int](560 * [TaskbarProbe]::GetDpiForWindow($display) / 96)
    }
    $rect.Top -= [Math]::Max(0,$CaptureAbove)
    $bitmap = New-Object Drawing.Bitmap(($rect.Right-$rect.Left),($rect.Bottom-$rect.Top))
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($rect.Left,$rect.Top,0,0,$bitmap.Size)
        $bitmap.Save($CapturePath,[Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
}
if ($ExerciseDisplay) { [TaskbarProbe]::SetCursorPos($previousCursor.X,$previousCursor.Y) | Out-Null }
if ($OutputPath) { Stop-Transcript | Out-Null }
