using System;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;
using System.Text;
using System.Threading;
using Microsoft.Win32.SafeHandles;

// Fixed publisher, version, digest and arguments. No caller-supplied download or execution target.
internal static class PawnIODownload
{
    internal const string Url = "https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/PawnIO_setup.exe";
    internal const string Sha256 = "1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032";
    internal const long MaxBytes = 16 * 1024 * 1024;
    private const int BudgetMs = 120000;

    internal static bool AllowedUri(Uri uri)
    {
        if (uri.Scheme != "https" || !uri.IsDefaultPort || uri.UserInfo != "" || uri.Fragment != "") return false;
        foreach (var host in new[] { "github.com", "release-assets.githubusercontent.com", "objects.githubusercontent.com", "github-releases.githubusercontent.com" })
            if (String.Equals(uri.DnsSafeHost, host, StringComparison.OrdinalIgnoreCase)) return true;
        return false;
    }

    internal static void CopyBounded(Stream source, Stream target, long declared, long limit, Stopwatch elapsed)
    {
        if (declared > limit) throw new InvalidDataException("官方驱动下载超过大小限制");
        long total = 0;
        var buffer = new byte[32768];
        while (true) {
            if (elapsed.ElapsedMilliseconds >= BudgetMs) throw new TimeoutException("官方驱动下载超时，请重试");
            if (source.CanTimeout) source.ReadTimeout = (int)Math.Max(1, Math.Min(15000, BudgetMs - elapsed.ElapsedMilliseconds));
            int count = source.Read(buffer, 0, buffer.Length);
            if (count == 0) break;
            total += count;
            if (total > limit) throw new InvalidDataException("官方驱动下载超过大小限制");
            target.Write(buffer, 0, count);
        }
        if (total == 0 || (declared >= 0 && total != declared)) throw new InvalidDataException("官方驱动下载不完整，请重试");
    }

    internal static void Download(Stream target)
    {
        ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
        Uri uri = new Uri(Url);
        var elapsed = Stopwatch.StartNew();
        for (int redirects = 0; redirects <= 5; redirects++) {
            if (!AllowedUri(uri)) throw new InvalidDataException("官方驱动下载跳转到未允许的地址");
            int remaining = BudgetMs - (int)elapsed.ElapsedMilliseconds;
            if (remaining <= 0) throw new TimeoutException("官方驱动下载超时，请重试");
            var request = (HttpWebRequest)WebRequest.Create(uri);
            request.AllowAutoRedirect = false;
            request.Timeout = Math.Min(20000, remaining);
            request.ReadWriteTimeout = Math.Min(15000, remaining);
            request.UserAgent = "Pinmeter/0.1.1";
            using (var response = (HttpWebResponse)request.GetResponse()) {
                int status = (int)response.StatusCode;
                if (status == 301 || status == 302 || status == 303 || status == 307 || status == 308) {
                    Uri next;
                    if (redirects == 5 || !Uri.TryCreate(uri, response.Headers["Location"], out next) || !AllowedUri(next))
                        throw new InvalidDataException("官方驱动下载跳转无效，请重试");
                    uri = next;
                    continue;
                }
                if (status != 200) throw new InvalidDataException("官方驱动下载未成功，请重试");
                using (var source = response.GetResponseStream()) CopyBounded(source, target, response.ContentLength, MaxBytes, elapsed);
                return;
            }
        }
        throw new InvalidDataException("官方驱动下载跳转次数过多");
    }

    internal static void VerifyHash(Stream stream)
    {
        stream.Position = 0;
        using (var hash = SHA256.Create()) {
            if (BitConverter.ToString(hash.ComputeHash(stream)).Replace("-", "") != Sha256)
                throw new InvalidDataException("驱动安装器校验失败，未执行安装，请重新下载");
        }
    }

    internal static void VerifySignature(string path)
    {
        const string script = "$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; try { if ((Get-AuthenticodeSignature -LiteralPath $env:PINMETER_DRIVER_DOWNLOAD).Status -ne 'Valid') { exit 1 }; exit 0 } catch { exit 1 }";
        var powershell = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), @"WindowsPowerShell\v1.0\powershell.exe");
        var start = new ProcessStartInfo(powershell, "-NoProfile -NonInteractive -EncodedCommand " + Convert.ToBase64String(Encoding.Unicode.GetBytes(script))) {
            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true
        };
        start.EnvironmentVariables["PINMETER_DRIVER_DOWNLOAD"] = path;
        start.EnvironmentVariables["PSModulePath"] = Path.Combine(Path.GetDirectoryName(powershell), "Modules");
        using (var process = Process.Start(start)) {
            if (process == null) throw new InvalidOperationException("无法检查驱动数字签名");
            process.OutputDataReceived += (sender, args) => {};
            process.ErrorDataReceived += (sender, args) => {};
            process.BeginOutputReadLine(); process.BeginErrorReadLine();
            if (!process.WaitForExit(20000)) {
                process.Kill(); process.WaitForExit();
                throw new TimeoutException("驱动数字签名检查超时，未执行安装");
            }
            process.WaitForExit();
            if (process.ExitCode != 0) throw new InvalidDataException("驱动数字签名无效，未执行安装");
        }
    }

    internal static string Complete(int exitCode, bool installed)
    {
        if (exitCode == 3010 || exitCode == 1641) return "驱动安装器要求重启电脑，请保存工作并手动重启后查看温度。";
        if (exitCode == 1223) throw new OperationCanceledException("已取消管理员授权，可重新下载安装");
        if (exitCode != 0) throw new InvalidOperationException("驱动安装失败（退出码 " + exitCode + "），请重试");
        if (!installed) throw new InvalidOperationException("安装器已结束，但尚未检测到 PawnIO，请重新检测或重启后检查");
        return "PawnIO 已安装，温度采集将在 30 秒内自动重试；部分设备可能需要重启电脑。";
    }

    internal static string Install()
    {
        using (var mutex = new Mutex(false, @"Local\Pinmeter.PawnIO.Install")) {
            bool acquired;
            try { acquired = mutex.WaitOne(0); } catch (AbandonedMutexException) { acquired = true; }
            if (!acquired) throw new InvalidOperationException("驱动正在下载或安装，请等待当前操作完成");
            string directory = null;
            bool directoryOwned = false;
            try {
                if (PawnIODriver.Installed()) return "PawnIO 已安装，无需重复下载安装；温度采集会自动重试。";
                using (var identity = WindowsIdentity.GetCurrent()) {
                    if (!new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator))
                        throw new InvalidOperationException("安装温度驱动需要管理员权限，请重新启动 Pinmeter 并允许授权");
                }
                var root = Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData);
                if ((File.GetAttributes(root) & FileAttributes.ReparsePoint) != 0) throw new IOException("驱动临时目录不可用");
                directory = Path.Combine(root, "Pinmeter-driver-" + Guid.NewGuid().ToString("N"));
                var admin = new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null);
                var system = new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null);
                var security = new DirectorySecurity();
                security.SetAccessRuleProtection(true, false);
                security.SetOwner(admin);
                foreach (var owner in new[] { admin, system })
                    security.AddAccessRule(new FileSystemAccessRule(owner, FileSystemRights.FullControl, InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit, PropagationFlags.None, AccessControlType.Allow));
                Directory.CreateDirectory(directory, security);
                using (LockDirectory(directory)) {
                    var actual = Directory.GetAccessControl(directory);
                    if (!actual.AreAccessRulesProtected || !actual.GetOwner(typeof(SecurityIdentifier)).Equals(admin))
                        throw new IOException("驱动临时目录权限不符合要求");
                    foreach (FileSystemAccessRule rule in actual.GetAccessRules(true, true, typeof(SecurityIdentifier))) {
                        if (rule.AccessControlType == AccessControlType.Allow && !rule.IdentityReference.Equals(admin) && !rule.IdentityReference.Equals(system))
                            throw new IOException("驱动临时目录不能允许其他账户写入");
                    }
                    directoryOwned = true;
                    string path = Path.Combine(directory, "PawnIO_setup.exe");
                    using (var download = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None)) {
                        Download(download);
                        download.Flush(true);
                    }
                    // Deny replacement/writes until installation exits, including signature checks.
                    using (var verified = OpenProtectedFile(path)) {
                        VerifyHash(verified);
                        VerifySignature(path);
                        using (var process = Process.Start(new ProcessStartInfo(path, "-install -silent") {
                            UseShellExecute = true, Verb = "runas", WindowStyle = ProcessWindowStyle.Hidden
                        })) {
                            if (process == null) throw new InvalidOperationException("无法启动驱动安装器");
                            // Never terminate a driver installer midway through system changes.
                            process.WaitForExit();
                            return Complete(process.ExitCode, PawnIODriver.Installed());
                        }
                    }
                }
            } finally {
                if (directoryOwned) {
                    try { File.Delete(Path.Combine(directory, "PawnIO_setup.exe")); Directory.Delete(directory, false); } catch { }
                }
                mutex.ReleaseMutex();
            }
        }
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFile(string name, uint access, uint share, IntPtr security, uint creation, uint flags, IntPtr template);

    internal static FileStream OpenProtectedFile(string path)
    {
        // Lock the path object itself, rather than following a replaceable symbolic link.
        var handle = CreateFile(path, 0x80000000, 1, IntPtr.Zero, 3, 0x00200000, IntPtr.Zero);
        if (handle.IsInvalid) { handle.Dispose(); throw new IOException("无法保护已下载的驱动文件"); }
        try {
            if ((File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0) throw new IOException("驱动文件路径不能是符号链接");
            return new FileStream(handle, FileAccess.Read);
        } catch { handle.Dispose(); throw; }
    }

    internal static SafeFileHandle LockDirectory(string directory)
    {
        // Open the directory itself (including reparse points) and deny rename/delete.
        var handle = CreateFile(directory, 0x81, 3, IntPtr.Zero, 3, 0x02200000, IntPtr.Zero);
        if (handle.IsInvalid) { handle.Dispose(); throw new IOException("无法保护驱动临时目录"); }
        if ((File.GetAttributes(directory) & FileAttributes.ReparsePoint) != 0) {
            handle.Dispose(); throw new IOException("驱动临时目录不能是目录联接");
        }
        return handle;
    }
}
