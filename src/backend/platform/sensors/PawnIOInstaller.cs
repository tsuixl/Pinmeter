using System;
using System.Diagnostics;
using System.IO;
using System.Security.Cryptography;
using System.Threading;
using Microsoft.Win32;

// Only fixed bundled installer + arguments; no path or command supplied by the UI.
internal static class PawnIOInstaller
{
    internal const string Sha256 = "1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032";

    internal static bool Installed()
    {
        // LHM caches this value in a static constructor; re-read after installation.
        using (var registry = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry64))
        using (var key = registry.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO")) {
            Version version;
            return key != null && Version.TryParse(key.GetValue("DisplayVersion") as string, out version);
        }
    }

    internal static void Verify(Stream stream)
    {
        using (var hash = SHA256.Create()) {
            if (BitConverter.ToString(hash.ComputeHash(stream)).Replace("-", "") != Sha256)
                throw new InvalidOperationException("PawnIO 安装器校验失败，请重新获取完整应用目录");
        }
    }

    internal static string Install()
    {
        using (var mutex = new Mutex(false, @"Local\Pinmeter.PawnIO.Install")) {
            bool acquired;
            try { acquired = mutex.WaitOne(0); }
            catch (AbandonedMutexException) { acquired = true; }
            if (!acquired) throw new InvalidOperationException("PawnIO 正在安装，请等待当前安装结束");
            try {
                if (Installed()) return "PawnIO 已安装，温度采集将在 30 秒内自动重试";
                var path = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "PawnIO_setup.exe");
                // Deny writes/deletes between hash validation and process completion.
                using (var file = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read)) {
                    Verify(file);
                    using (var process = Process.Start(new ProcessStartInfo(path, "-install -silent") {
                        UseShellExecute = true, Verb = "runas", WindowStyle = ProcessWindowStyle.Hidden
                    })) {
                        if (process == null) throw new InvalidOperationException("无法启动 PawnIO 安装器");
                        // Do not kill an in-progress driver installation or release the mutex early.
                        process.WaitForExit();
                        if (process.ExitCode == 3010 || process.ExitCode == 1641)
                            return "PawnIO 安装器要求重启电脑，请重启后查看温度";
                        if (process.ExitCode != 0)
                            throw new InvalidOperationException("PawnIO 安装失败（退出码 " + process.ExitCode + "），请重试");
                    }
                }
                if (!Installed()) throw new InvalidOperationException("安装器已退出，但未检测到 PawnIO，请重试或重启电脑后检查");
                return "PawnIO 已安装，温度采集将在 30 秒内自动重试；若仍提示驱动无法访问，请重启电脑";
            }
            finally { mutex.ReleaseMutex(); }
        }
    }

    internal static int Run(bool install)
    {
        try {
            var detail = install ? Install() : (Installed() ? "installed" : "missing");
            Console.WriteLine(new System.Web.Script.Serialization.JavaScriptSerializer().Serialize(new { ok = true, detail }));
            return 0;
        }
        catch (Exception error) {
            var win32 = error as System.ComponentModel.Win32Exception;
            var detail = win32 != null && win32.NativeErrorCode == 1223 ? "已取消管理员授权，可重新安装" :
                error is FileNotFoundException ? "缺少 PawnIO 安装器，请复制完整应用目录后重试" : error.Message;
            Console.WriteLine(new System.Web.Script.Serialization.JavaScriptSerializer().Serialize(new { ok = false, detail }));
            return 1;
        }
    }
}
