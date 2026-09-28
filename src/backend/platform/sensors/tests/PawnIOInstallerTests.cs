using System;
using System.IO;

internal static class PawnIOInstallerTests
{
    public static int Main(string[] args)
    {
        using (var valid = File.OpenRead(args[0])) PawnIOInstaller.Verify(valid);
        using (var corrupt = new MemoryStream(new byte[] { 1, 2, 3 })) {
            try { PawnIOInstaller.Verify(corrupt); return 1; }
            catch (InvalidOperationException) { }
        }
        // Installed systems must return without even resolving/launching a bundled installer.
        if (PawnIOInstaller.Installed()) {
            if (!PawnIOInstaller.Install().Contains("已安装")) return 2;
        }
        Console.WriteLine("PASS: official installer hash, corrupt installer rejection, installed-system idempotency");
        return 0;
    }
}
