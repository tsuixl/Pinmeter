using System;
using Microsoft.Win32;

// Read-only detection and the explicit fixed-source installation command.
internal static class PawnIODriver
{
    internal static bool Installed()
    {
        using (var registry = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry64))
        using (var key = registry.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO")) {
            Version version;
            return key != null && Version.TryParse(key.GetValue("DisplayVersion") as string, out version);
        }
    }

    internal static int Run(bool install)
    {
        try {
            string detail = install ? PawnIODownload.Install() : (Installed() ? "installed" : "missing");
            Console.WriteLine(new System.Web.Script.Serialization.JavaScriptSerializer().Serialize(new { ok = true, detail }));
            return 0;
        } catch (Exception error) {
            var win32 = error as System.ComponentModel.Win32Exception;
            string detail = win32 != null && win32.NativeErrorCode == 1223 ? "已取消管理员授权，可重新下载安装" :
                error is System.Net.WebException ? "官方驱动下载失败，请检查网络后重试" : error.Message;
            Console.WriteLine(new System.Web.Script.Serialization.JavaScriptSerializer().Serialize(
                new { ok = false, detail }));
            return 1;
        }
    }
}
