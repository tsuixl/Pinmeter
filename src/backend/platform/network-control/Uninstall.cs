using System;
using System.Diagnostics;
using System.IO;
using System.Text;

// Only the explicit uninstaller command removes startup registration.
internal static class Uninstall
{
    internal static int Cleanup(Func<int> releaseNetwork, Func<int> removeStartup)
    {
        int network = releaseNetwork();
        return network == 0 ? removeStartup() : network;
    }

    internal static int RemoveStartup()
    {
        try {
            string script;
            using (var stream = typeof(Uninstall).Assembly.GetManifestResourceStream("autostart.ps1"))
            using (var reader = new StreamReader(stream)) script = reader.ReadToEnd();
            string powershell = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), @"WindowsPowerShell\v1.0\powershell.exe");
            var start = new ProcessStartInfo(powershell, "-NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand " + Convert.ToBase64String(Encoding.Unicode.GetBytes(script))) {
                UseShellExecute = false, CreateNoWindow = true
            };
            start.EnvironmentVariables["PINMETER_AUTOSTART_ACTION"] = "uninstall";
            start.EnvironmentVariables["PINMETER_AUTOSTART_EXE"] = Path.Combine(Program.Root, "Pinmeter.exe");
            start.EnvironmentVariables.Remove("PINMETER_AUTOSTART_CHECKPOINT");
            using (var process = Process.Start(start)) {
                if (process == null) return 1;
                if (!process.WaitForExit(15000)) { process.Kill(); process.WaitForExit(); return 1; }
                return process.ExitCode;
            }
        } catch (Exception error) {
            Console.Error.WriteLine("Startup cleanup failed: " + error.Message);
            return 1;
        }
    }

    internal static void SelfTest()
    {
        bool startup = false;
        if (Cleanup(() => 5, () => { startup = true; return 0; }) != 5 || startup)
            throw new Exception("Failed network recovery must stop uninstall cleanup");
        if (Cleanup(() => 0, () => 1) != 1)
            throw new Exception("Startup cleanup failure must stop uninstall");
        if (Cleanup(() => 0, () => 0) != 0)
            throw new Exception("Successful uninstall cleanup rejected");
        using (var resource = typeof(Uninstall).Assembly.GetManifestResourceStream("autostart.ps1"))
            if (resource == null) throw new Exception("Startup cleanup resource missing");
    }
}
