using System;
using Microsoft.Win32;

// Detection only. The user obtains and installs the driver from its publisher.
internal static class PawnIODriver
{
    internal static int Status()
    {
        try {
            using (var registry = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry64))
            using (var key = registry.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO")) {
                Version version;
                var installed = key != null && Version.TryParse(key.GetValue("DisplayVersion") as string, out version);
                Console.WriteLine(new System.Web.Script.Serialization.JavaScriptSerializer().Serialize(
                    new { ok = true, detail = installed ? "installed" : "missing" }));
                return 0;
            }
        } catch (Exception error) {
            Console.WriteLine(new System.Web.Script.Serialization.JavaScriptSerializer().Serialize(
                new { ok = false, detail = error.Message }));
            return 1;
        }
    }
}
