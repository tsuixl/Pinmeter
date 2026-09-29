using System;
using System.Diagnostics;
using System.IO;

internal static class DriverDownloadTests
{
    private static void Reject(Action action)
    {
        bool rejected = false;
        try { action(); } catch { rejected = true; }
        if (!rejected) throw new Exception("Unsafe driver operation was accepted");
    }

    private static int Main(string[] args)
    {
        try {
            if (args.Length == 2 && args[0] == "--download-check") {
                // Explicit verification probe only. It never runs an installer.
                Directory.CreateDirectory(args[1]);
                string file = Path.Combine(args[1], "verified-download.exe");
                using (var download = new FileStream(file, FileMode.CreateNew, FileAccess.Write, FileShare.None)) {
                    PawnIODownload.Download(download); download.Flush(true);
                }
                using (var verified = new FileStream(file, FileMode.Open, FileAccess.Read, FileShare.Read)) {
                    PawnIODownload.VerifyHash(verified);
                    PawnIODownload.VerifySignature(file);
                    Console.WriteLine("PASS: official HTTPS download, SHA-256 and Authenticode signature; bytes=" + verified.Length + "; installer not executed");
                }
                return 0;
            }
            if (args.Length != 0) return 2;
            foreach (var uri in new[] { PawnIODownload.Url, "https://release-assets.githubusercontent.com/download?signature=example", "https://objects.githubusercontent.com/download" })
                if (!PawnIODownload.AllowedUri(new Uri(uri))) throw new Exception("Official HTTPS target rejected");
            foreach (var uri in new[] { "http://github.com/download", "https://github.com.evil.invalid/download", "https://evil.invalid/", "https://user@github.com/download", "https://github.com:444/download", "file:///C:/test.exe", "https://github.com/download#fragment" })
                if (PawnIODownload.AllowedUri(new Uri(uri))) throw new Exception("Untrusted redirect accepted");
            using (var output = new MemoryStream()) {
                PawnIODownload.CopyBounded(new MemoryStream(new byte[] { 1, 2, 3 }), output, 3, 3, Stopwatch.StartNew());
                if (output.Length != 3) throw new Exception("Bounded copy lost bytes");
            }
            Reject(() => PawnIODownload.CopyBounded(new MemoryStream(new byte[5]), new MemoryStream(), 5, 4, Stopwatch.StartNew()));
            Reject(() => PawnIODownload.CopyBounded(new MemoryStream(new byte[5]), new MemoryStream(), -1, 4, Stopwatch.StartNew()));
            Reject(() => PawnIODownload.CopyBounded(new MemoryStream(new byte[3]), new MemoryStream(), 5, 8, Stopwatch.StartNew()));
            Reject(() => PawnIODownload.CopyBounded(new MemoryStream(), new MemoryStream(), -1, 8, Stopwatch.StartNew()));
            Reject(() => PawnIODownload.VerifyHash(new MemoryStream(new byte[] { 1, 2, 3 })));
            Reject(() => PawnIODownload.Complete(1, true));
            Reject(() => PawnIODownload.Complete(1223, false));
            Reject(() => PawnIODownload.Complete(0, false));
            if (!PawnIODownload.Complete(3010, false).Contains("重启") || !PawnIODownload.Complete(1641, true).Contains("重启") || !PawnIODownload.Complete(0, true).Contains("已安装"))
                throw new Exception("Installer result semantics regressed");
            string directory = Path.Combine(Path.GetTempPath(), "pinmeter-driver-test-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(directory);
            string sample = Path.Combine(directory, "unsigned.exe");
            try {
                File.WriteAllText(sample, "unsigned fixture");
                using (PawnIODownload.LockDirectory(directory)) {
                    Reject(() => Directory.Move(directory, directory + "-moved"));
                    using (var locked = new FileStream(sample, FileMode.Open, FileAccess.Read, FileShare.Read)) {
                        Reject(() => File.WriteAllText(sample, "changed"));
                        Reject(() => File.Delete(sample));
                        Reject(() => PawnIODownload.VerifySignature(sample));
                    }
                }
            } finally {
                foreach (var temporary in new[] { directory, directory + "-moved" }) {
                    if (Directory.Exists(temporary)) { File.Delete(Path.Combine(temporary, "unsigned.exe")); Directory.Delete(temporary, false); }
                }
            }
            Console.WriteLine("PASS: official redirect allowlist, bounded/truncated downloads, digest rejection, unsigned rejection, file/directory locks, cancellation/reboot and installation verification");
            return 0;
        } catch (Exception error) { Console.Error.WriteLine(error); return 1; }
    }
}
