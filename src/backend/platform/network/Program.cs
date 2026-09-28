using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Principal;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;
using Microsoft.Win32.SafeHandles;

internal static class Program
{
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetNamedPipeClientProcessId(IntPtr pipe, out uint pid);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetNamedPipeServerProcessId(IntPtr pipe, out uint pid);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern SafeFileHandle OpenProcess(uint access, bool inherit, uint pid);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool QueryFullProcessImageName(SafeFileHandle process, uint flags, StringBuilder name, ref int size);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetProcessTimes(SafeFileHandle process, out long creation, out long exit, out long kernel, out long user);
    private const int MaxPacket = 2 * 1024 * 1024;
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer { MaxJsonLength = MaxPacket };
    private static NativeEtw trace;
    private static readonly object Gate = new object();
    private static readonly object SessionGate = new object();
    private static readonly Dictionary<string, string> Icons = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private sealed class Bytes { internal ulong down, up; internal long first; }
    private static Dictionary<uint, Bytes> counters = new Dictionary<uint, Bytes>();
    private static ulong overflowDown, overflowUp;
    private static ulong sequence;
    private static readonly Stopwatch Clock = Stopwatch.StartNew();
    private static long previous;
    private static void Reply(object packet) { Console.WriteLine(Json.Serialize(packet)); Console.Out.Flush(); }
    private static void Error(string status, string detail) { Reply(new { status, detail }); }
    private static string ReadBounded(TextReader reader, int limit) {
        var value = new StringBuilder();
        for (int i = 0; i < limit; i++) {
            int c = reader.Read();
            if (c < 0) return null;
            if (c == '\n') return value.ToString().TrimEnd('\r');
            value.Append((char)c);
        }
        throw new InvalidDataException("Protocol message too large");
    }
    private static int Broker(Process parent) {
        string name = "Pinmeter-network-" + Guid.NewGuid().ToString("N");
        var acl = new PipeSecurity();
        acl.SetAccessRuleProtection(true, false);
        acl.AddAccessRule(new PipeAccessRule(WindowsIdentity.GetCurrent().User, PipeAccessRights.FullControl, AccessControlType.Allow));
        acl.AddAccessRule(new PipeAccessRule(new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null), PipeAccessRights.FullControl, AccessControlType.Allow));
        using (var pipe = new NamedPipeServerStream(name, PipeDirection.InOut, 1, PipeTransmissionMode.Byte, PipeOptions.Asynchronous, 4096, 4096, acl)) {
            Process worker;
            try { worker = Process.Start(new ProcessStartInfo(typeof(Program).Assembly.Location,
                Process.GetCurrentProcess().Id + " --pipe " + name) { UseShellExecute = true, Verb = "runas", WindowStyle = ProcessWindowStyle.Hidden }); }
            catch (Win32Exception) { Error("permission_denied", "未授权读取应用网络流量"); return 1; }
            using (worker) {
                var connected = pipe.BeginWaitForConnection(null, null);
                if (!connected.AsyncWaitHandle.WaitOne(10000)) return 2;
                pipe.EndWaitForConnection(connected);
                uint pid;
                if (!GetNamedPipeClientProcessId(pipe.SafePipeHandle.DangerousGetHandle(), out pid) || pid != worker.Id) return 2;
                using (var input = new StreamReader(pipe))
                using (var output = new StreamWriter(pipe) { AutoFlush = true }) {
                    string command;
                    while ((command = ReadBounded(Console.In, 32)) != null) {
                        if (command != "sample" && command != "stop" && command != "shutdown") break;
                        output.WriteLine(command);
                        string response = ReadBounded(input, MaxPacket);
                        if (response == null) break;
                        Console.WriteLine(response); Console.Out.Flush();
                        if (command == "shutdown") break;
                    }
                }
            }
        }
        return 0;
    }
    private static void Receive(uint pid, uint size, bool upload, long timestamp) {
        lock (Gate) {
            Bytes bytes;
            if (!counters.TryGetValue(pid, out bytes)) {
                if (counters.Count >= 4096) { if (upload) overflowUp += size; else overflowDown += size; return; }
                bytes = new Bytes { first = timestamp }; counters.Add(pid, bytes);
            }
            if (timestamp < bytes.first) bytes.first = timestamp;
            if (upload) bytes.up += size; else bytes.down += size;
        }
    }
    private static void StopCollection() {
        lock (SessionGate) {
            if (trace != null) { trace.Dispose(); trace = null; }
            lock (Gate) {
                counters.Clear(); overflowDown = overflowUp = 0;
                sequence = 0; previous = Clock.ElapsedMilliseconds;
            }
        }
    }
    private static object Snapshot() {
        ulong lost = trace.Lost();
        Dictionary<uint, Bytes> batch;
        ulong unknownDown, unknownUp;
        long now;
        lock (Gate) {
            batch = counters; counters = new Dictionary<uint, Bytes>();
            unknownDown = overflowDown; unknownUp = overflowUp; overflowDown = overflowUp = 0;
            now = Clock.ElapsedMilliseconds;
        }
        var rows = new List<object>();
        var iconPaths = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        ulong otherDown = 0, otherUp = 0;
        foreach (var item in batch) {
            string path = null; long created = 0;
            using (var process = OpenProcess(0x1000, false, item.Key)) {
                if (!process.IsInvalid) {
                    long exit, kernel, user;
                    int size = 1024; var buffer = new StringBuilder(size);
                    if (GetProcessTimes(process, out created, out exit, out kernel, out user)
                        && created <= item.Value.first && exit == 0
                        && QueryFullProcessImageName(process, 0, buffer, ref size)) path = buffer.ToString();
                }
            }
            if (path == null) { unknownDown += item.Value.down; unknownUp += item.Value.up; continue; }
            if (rows.Count >= 512) { otherDown += item.Value.down; otherUp += item.Value.up; continue; }
            rows.Add(new { pid = item.Key, started = created.ToString(CultureInfo.InvariantCulture), path,
                icon = iconPaths.Add(path) ? IconFor(path) : null,
                download = item.Value.down.ToString(CultureInfo.InvariantCulture), upload = item.Value.up.ToString(CultureInfo.InvariantCulture) });
        }
        long elapsed = now - previous; previous = now;
        return new { status = "normal", sequence = (++sequence).ToString(CultureInfo.InvariantCulture), elapsed_ms = elapsed,
            lost = lost.ToString(CultureInfo.InvariantCulture), rows,
            unknown_download = unknownDown.ToString(CultureInfo.InvariantCulture), unknown_upload = unknownUp.ToString(CultureInfo.InvariantCulture),
            other_download = otherDown.ToString(CultureInfo.InvariantCulture), other_upload = otherUp.ToString(CultureInfo.InvariantCulture) };
    }
    private static string IconFor(string path) {
        string encoded;
        if (Icons.TryGetValue(path, out encoded)) return encoded;
        if (Icons.Count >= 128) return null;
        try {
            if (path.Length > 2 && path[1] == ':') {
                using (var icon = System.Drawing.Icon.ExtractAssociatedIcon(path))
                using (var small = new System.Drawing.Icon(icon, 16, 16))
                using (var bitmap = small.ToBitmap())
                using (var stream = new MemoryStream()) {
                    bitmap.Save(stream, System.Drawing.Imaging.ImageFormat.Png);
                    if (stream.Length <= 4096) encoded = Convert.ToBase64String(stream.ToArray());
                }
            }
        } catch { encoded = null; }
        Icons[path] = encoded;
        return encoded;
    }
    public static int Main(string[] args) {
        Console.InputEncoding = Console.OutputEncoding = new UTF8Encoding(false);
        int parentId;
        if (args.Length < 1 || !int.TryParse(args[0], out parentId)) return 2;
        Guid identity = Guid.NewGuid();
        bool ownedSession = args.Length == 3 && args[1] == "--session";
        if (ownedSession && (!Guid.TryParseExact(args[2], "N", out identity) || identity == Guid.Empty)) return 2;
        Process parent;
        try { parent = Process.GetProcessById(parentId); } catch { return 2; }
        var watcher = new Thread(() => {
            try { parent.WaitForExit(); } catch { return; }
            try { StopCollection(); Environment.Exit(0); }
            catch (Exception error) { Console.Error.WriteLine("ETW parent-exit cleanup failed: " + error); Environment.Exit(1); }
        });
        watcher.IsBackground = true; watcher.Start();
        NamedPipeClientStream pipe = null;
        try {
            if (args.Length == 2 && args[1] == "--elevate") return Broker(parent);
            if (args.Length == 3 && args[1] == "--pipe") {
                pipe = new NamedPipeClientStream(".", args[2], PipeDirection.InOut);
                pipe.Connect(10000);
                uint pid;
                if (!GetNamedPipeServerProcessId(pipe.SafePipeHandle.DangerousGetHandle(), out pid) || pid != parentId) return 2;
                Console.SetIn(new StreamReader(pipe));
                Console.SetOut(new StreamWriter(pipe) { AutoFlush = true });
            } else if (args.Length != 1 && !ownedSession) return 2;
            string command;
            while ((command = ReadBounded(Console.In, 32)) != null) {
                if (command == "shutdown") {
                    StopCollection();
                    Reply(new { status = "shutdown_complete" });
                    break;
                }
                if (command == "stop") {
                    StopCollection();
                    Reply(new { status = "stopped" });
                    continue;
                }
                if (command != "sample") break;
                if (trace == null) {
                    lock (SessionGate) {
                        if (parent.HasExited) return 0;
                        trace = new NativeEtw(identity); trace.OnBytes = Receive; trace.Start(); previous = Clock.ElapsedMilliseconds;
                    }
                }
                Reply(Snapshot());
            }
            StopCollection(); // EOF also must finish cleanup before Main reports success.
            return 0;
        } catch (Win32Exception error) {
            Error(error.NativeErrorCode == 5 ? "permission_denied" : "failed", "应用网络采集失败（Windows " + error.NativeErrorCode + "）"); return 1;
        } catch (Exception error) { Error("failed", "应用网络采集失败：" + error.GetType().Name); return 1; }
        finally {
            try { StopCollection(); }
            catch (Exception error) { Console.Error.WriteLine("ETW cleanup failed: " + error); Environment.ExitCode = 1; }
            if (pipe != null) pipe.Dispose(); parent.Dispose();
        }
    }
}
