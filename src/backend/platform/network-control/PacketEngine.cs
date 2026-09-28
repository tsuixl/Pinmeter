using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using Microsoft.Win32.SafeHandles;

// API declarations follow WinDivert 2.2.2 (LGPLv3); see THIRD-PARTY-NOTICES.txt.
internal static class Divert {
    [DllImport("WinDivert.dll", CallingConvention = CallingConvention.Cdecl, SetLastError = true)] internal static extern IntPtr WinDivertOpen(string filter, int layer, short priority, ulong flags);
    [DllImport("WinDivert.dll", CallingConvention = CallingConvention.Cdecl, SetLastError = true)] internal static extern bool WinDivertRecv(IntPtr h, byte[] packet, uint length, out uint received, byte[] address);
    [DllImport("WinDivert.dll", CallingConvention = CallingConvention.Cdecl, SetLastError = true)] internal static extern bool WinDivertSend(IntPtr h, byte[] packet, uint length, out uint sent, byte[] address);
    [DllImport("WinDivert.dll", CallingConvention = CallingConvention.Cdecl)] internal static extern bool WinDivertClose(IntPtr h);
    [DllImport("WinDivert.dll", CallingConvention = CallingConvention.Cdecl)] internal static extern bool WinDivertShutdown(IntPtr h, int how);
    [DllImport("WinDivert.dll", CallingConvention = CallingConvention.Cdecl)] internal static extern bool WinDivertHelperCalcChecksums(byte[] packet, uint length, byte[] address, ulong flags);
    [DllImport("WinDivert.dll", CallingConvention = CallingConvention.Cdecl)] internal static extern bool WinDivertHelperFormatIPv6Address(uint[] address, StringBuilder text, uint length);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] internal static extern bool SetDllDirectory(string path);
    [DllImport("iphlpapi.dll")] internal static extern uint GetExtendedTcpTable(IntPtr table, ref uint size, bool sorted, int family, int type, uint reserved);
    [DllImport("iphlpapi.dll")] internal static extern uint GetExtendedUdpTable(IntPtr table, ref uint size, bool sorted, int family, int type, uint reserved);
    [DllImport("kernel32.dll", SetLastError = true)] internal static extern SafeFileHandle OpenProcess(uint access, bool inherit, uint pid);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] internal static extern bool QueryFullProcessImageName(SafeFileHandle h, uint flags, StringBuilder name, ref int length);
}

internal sealed class PacketEngine : IDisposable {
    internal static double Due(double previous, double now, int length, uint rate) { return Math.Max(previous, now) + length * 1000.0 / rate; }
    internal static void SelfTest() {
        double first = Due(0, 0, 65535, 16000), next = Due(first, 1, 1500, 16000);
        if (first < 4095 || next <= first || Due(next, 10000, 16000, 16000) != 11000) throw new Exception("Pacing debt/burst regression");
        double finish = 0; for (int i = 0; i < 1000; i++) finish = Due(finish, 0, 1500, 32000);
        if (Math.Abs(finish - 46875) > 0.001) throw new Exception("Shared budget regression");
        var ipv4 = new byte[40]; ipv4[0] = 0x45; ipv4[9] = 6; ipv4[12] = 10; ipv4[15] = 1; ipv4[16] = 10; ipv4[19] = 2; ipv4[20] = 1; ipv4[21] = 2; ipv4[22] = 3; ipv4[23] = 4;
        string exact, local, wildcard;
        if (!PacketKeys(ipv4, 40, true, out exact, out local, out wildcard) || exact != "6|10.0.0.1|258|10.0.0.2|772") throw new Exception("IPv4 attribution regression");
        if (!PacketKeys(ipv4, 40, false, out exact, out local, out wildcard) || exact != "6|10.0.0.2|772|10.0.0.1|258") throw new Exception("Direction regression");
        ipv4[6] = 32; if (PacketKeys(ipv4, 40, true, out exact, out local, out wildcard)) throw new Exception("Fragment must not be misattributed");
        var ipv6 = new byte[48]; ipv6[0] = 0x60; ipv6[6] = 17; ipv6[23] = 1; ipv6[39] = 2; ipv6[41] = 1; ipv6[43] = 2;
        if (!PacketKeys(ipv6, 48, true, out exact, out local, out wildcard) || exact != "17|::1|1|::2|2" || wildcard != "17|::|1") throw new Exception("IPv6 attribution regression");
        bool rejected = false; try { Program.Validate(new Rule { id = "self", path = typeof(Program).Assembly.Location }); } catch { rejected = true; }
        if (!rejected) throw new Exception("Own helper must be protected");
    }
    private readonly object gate = new object();
    private readonly object ownersGate = new object();
    private readonly Stopwatch clock = Stopwatch.StartNew();
    private IntPtr network = IntPtr.Zero, flow = IntPtr.Zero;
    private volatile bool running;
    private Thread captureThread, drainThread, flowThread, tableThread;
    private Dictionary<string, Rule> rules;
    private Dictionary<string, string> owners = new Dictionary<string, string>();
    private readonly Dictionary<string, string> recent = new Dictionary<string, string>();
    private readonly Dictionary<string, Bucket> buckets = new Dictionary<string, Bucket>(StringComparer.OrdinalIgnoreCase);
    private int queuedBytes;
    private readonly string diagnosticId = Guid.NewGuid().ToString("N");
    private long received, sentPackets, droppedPackets, lastReceiveMs, lastSendMs;
    private string lastError = "";
    internal object DiagnosticState() {
        // Avoid gate: a stuck packet operation must not also stall diagnostic reads.
        return new { id = diagnosticId, running, network_open = network != IntPtr.Zero, flow_open = flow != IntPtr.Zero,
            received = Interlocked.Read(ref received), sent = Interlocked.Read(ref sentPackets),
            capture_dropped = Interlocked.Read(ref droppedPackets), queued_bytes = Volatile.Read(ref queuedBytes),
            elapsed_ms = clock.ElapsedMilliseconds, last_receive_ms = Interlocked.Read(ref lastReceiveMs),
            last_send_ms = Interlocked.Read(ref lastSendMs), error = lastError };
    }
    private sealed class Packet { internal byte[] data, address; internal double due; }
    private sealed class Bucket { internal double next; internal int bytes; internal Queue<Packet> queue = new Queue<Packet>(); }
    internal bool Healthy { get { return running; } }
    internal static bool FilesPresent { get { return File.Exists(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "WinDivert.dll")) && File.Exists(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "WinDivert64.sys")); } }
    internal PacketEngine(Rule[] initial) {
        Diagnostics.Record("engine-create", new { id = diagnosticId, rule_count = initial.Length });
        rules = initial.ToDictionary(r => r.path, StringComparer.OrdinalIgnoreCase);
        if (!FilesPresent) throw new Exception("缺少 WinDivert 驱动资源");
        Divert.SetDllDirectory(AppDomain.CurrentDomain.BaseDirectory);
        try {
            flow = Divert.WinDivertOpen("(tcp or udp)", 2, 0, 5);
            if (flow == new IntPtr(-1)) { flow = IntPtr.Zero; throw new Win32Exception(); }
            Diagnostics.Record("flow-open", new { id = diagnosticId });
            RefreshOwners();
            network = Divert.WinDivertOpen("(tcp or udp) and !loopback and !impostor", 0, 0, 0);
            if (network == new IntPtr(-1)) { network = IntPtr.Zero; throw new Win32Exception(); }
            Diagnostics.Record("network-open", new { id = diagnosticId });
            running = true;
            captureThread = Start(Capture); drainThread = Start(Drain); flowThread = Start(Flows); tableThread = Start(Tables);
        } catch (Exception e) { lastError = e.ToString(); Diagnostics.Record("engine-create-error", DiagnosticState()); Dispose(); throw; }
    }
    private Thread Start(ThreadStart action) {
        var t = new Thread(() => {
            try { action(); }
            catch (Exception e) {
                lastError = e.ToString(); Diagnostics.Record("engine-worker-error", new { worker = action.Method.Name, state = DiagnosticState() });
                StopHandles();
            }
        }) { IsBackground = true }; t.Start(); return t;
    }
    private void StopHandles() {
        running = false;
        lock (gate) {
            if (network != IntPtr.Zero) {
                bool shutdown = Divert.WinDivertShutdown(network, 3), closed = Divert.WinDivertClose(network);
                network = IntPtr.Zero; Diagnostics.Record("network-close", new { id = diagnosticId, shutdown, closed });
            }
            if (flow != IntPtr.Zero) {
                bool shutdown = Divert.WinDivertShutdown(flow, 3), closed = Divert.WinDivertClose(flow);
                flow = IntPtr.Zero; Diagnostics.Record("flow-close", new { id = diagnosticId, shutdown, closed });
            }
        }
    }
    public void Dispose() {
        Diagnostics.Record("engine-dispose-begin", DiagnosticState());
        StopHandles();
        foreach (var t in new Thread[] { captureThread, drainThread, flowThread, tableThread })
            if (t != null && t != Thread.CurrentThread && !t.Join(1500)) Diagnostics.Record("engine-join-timeout", new { id = diagnosticId });
        lock (gate) { buckets.Clear(); queuedBytes = 0; }
        Diagnostics.Record("engine-dispose-complete", DiagnosticState());
    }
    internal void Update(Rule[] next) {
        lock (gate) {
            var updated = next.ToDictionary(r => r.path, StringComparer.OrdinalIgnoreCase);
            foreach (var entry in buckets.ToArray()) {
                string path = entry.Key.Substring(0, entry.Key.Length - 2); Rule rule;
                bool exists = updated.TryGetValue(path, out rule);
                uint? rate = exists ? (entry.Key.EndsWith("|u") ? rule.upload : rule.download) : null;
                if (exists && !rule.blocked && rate.HasValue) {
                    Rule previous;
                    uint? oldRate = rules.TryGetValue(path, out previous) ? (entry.Key.EndsWith("|u") ? previous.upload : previous.download) : null;
                    if (oldRate == rate) continue;
                    double now = clock.Elapsed.TotalMilliseconds, due = now;
                    // Repace existing packets on edits; do not release an unrelated application's queue.
                    var retained = new Queue<Packet>();
                    foreach (var p in entry.Value.queue) {
                        double planned = Due(due, now, p.data.Length, rate.Value);
                        if (planned - now > 5000) { queuedBytes -= p.data.Length; entry.Value.bytes -= p.data.Length; continue; }
                        due = planned; p.due = due; retained.Enqueue(p);
                    }
                    entry.Value.queue = retained;
                    entry.Value.next = due;
                } else {
                    if (!exists || !rule.blocked) foreach (var p in entry.Value.queue) Send(p);
                    queuedBytes -= entry.Value.bytes; buckets.Remove(entry.Key);
                }
            }
            rules = updated;
        }
    }
    private void Send(Packet p) {
        if (network == IntPtr.Zero) return;
        uint sent;
        if (!Divert.WinDivertHelperCalcChecksums(p.data, (uint)p.data.Length, p.address, 0)
            || !Divert.WinDivertSend(network, p.data, (uint)p.data.Length, out sent, p.address)) {
            throw new Win32Exception(Marshal.GetLastWin32Error(), "数据包重注入失败");
        }
        Interlocked.Increment(ref sentPackets); Interlocked.Exchange(ref lastSendMs, clock.ElapsedMilliseconds);
    }
    private void Capture() {
        var buffer = new byte[65535]; var address = new byte[80];
        while (running) {
            uint length;
            if (!Divert.WinDivertRecv(network, buffer, (uint)buffer.Length, out length, address)) { if (running) throw new Win32Exception(); break; }
            Interlocked.Increment(ref received); Interlocked.Exchange(ref lastReceiveMs, clock.ElapsedMilliseconds);
            bool outbound = (address[10] & 2) != 0;
            string exact, local, wildcard;
            string path = null;
            if (PacketKeys(buffer, (int)length, outbound, out exact, out local, out wildcard)) lock (ownersGate) {
                if (!recent.TryGetValue(exact, out path) && !owners.TryGetValue(exact, out path)
                    && !owners.TryGetValue(local, out path)) owners.TryGetValue(wildcard, out path);
            }
            var p = new Packet { data = new byte[length], address = (byte[])address.Clone() };
            Buffer.BlockCopy(buffer, 0, p.data, 0, (int)length);
            lock (gate) {
                Rule r;
                if (path == null || !rules.TryGetValue(path, out r)) { Send(p); continue; }
                if (r.blocked) { Interlocked.Increment(ref droppedPackets); continue; }
                uint? rate = outbound ? r.upload : r.download;
                if (!rate.HasValue) { Send(p); continue; }
                string key = path + (outbound ? "|u" : "|d"); Bucket bucket;
                if (!buckets.TryGetValue(key, out bucket)) { bucket = new Bucket(); buckets.Add(key, bucket); }
                double now = clock.Elapsed.TotalMilliseconds;
                double due = Due(bucket.next, now, (int)length, rate.Value);
                if (due - now > 5000 || bucket.queue.Count >= 1024 || bucket.bytes + length > 1048576 || queuedBytes + length > 16777216) {
                    Interlocked.Increment(ref droppedPackets); continue;
                }
                bucket.next = due; p.due = due; bucket.queue.Enqueue(p); bucket.bytes += (int)length; queuedBytes += (int)length;
            }
        }
    }
    private void Drain() {
        while (running) {
            lock (gate) {
                double now = clock.Elapsed.TotalMilliseconds;
                foreach (var bucket in buckets.Values) {
                    int budget = 64;
                    while (budget-- > 0 && bucket.queue.Count > 0 && bucket.queue.Peek().due <= now) {
                        var p = bucket.queue.Dequeue(); bucket.bytes -= p.data.Length; queuedBytes -= p.data.Length; Send(p);
                    }
                }
            }
            Thread.Sleep(2);
        }
    }
    private static string Ip(byte[] data, int offset, int length) {
        var bytes = new byte[length]; Buffer.BlockCopy(data, offset, bytes, 0, length); return NormalIp(new IPAddress(bytes));
    }
    private static string NormalIp(IPAddress ip) { return ip.IsIPv4MappedToIPv6 ? ip.MapToIPv4().ToString() : ip.ToString(); }
    private static int Port(byte[] data, int offset) { return data[offset] * 256 + data[offset + 1]; }
    private static string LocalKey(int protocol, string ip, int port) { return protocol + "|" + ip + "|" + port; }
    private static bool PacketKeys(byte[] data, int length, bool outbound, out string exact, out string local, out string wildcard) {
        exact = local = wildcard = null;
        if (length < 20) return false;
        int offset, protocol, version = data[0] >> 4; string src, dst;
        if (version == 4) {
            offset = (data[0] & 15) * 4; if (offset < 20 || (data[6] & 63) != 0 || data[7] != 0) return false;
            protocol = data[9]; src = Ip(data, 12, 4); dst = Ip(data, 16, 4);
        } else if (version == 6 && length >= 40) {
            offset = 40; protocol = data[6]; src = Ip(data, 8, 16); dst = Ip(data, 24, 16);
            for (int i = 0; i < 8 && (protocol == 0 || protocol == 43 || protocol == 60 || protocol == 51); i++) {
                if (offset + 2 > length) return false;
                int size = protocol == 51 ? (data[offset + 1] + 2) * 4 : (data[offset + 1] + 1) * 8;
                protocol = data[offset]; offset += size;
            }
        } else return false;
        if ((protocol != 6 && protocol != 17) || offset + (protocol == 6 ? 20 : 8) > length) return false;
        int sp = Port(data, offset), dp = Port(data, offset + 2);
        local = LocalKey(protocol, outbound ? src : dst, outbound ? sp : dp);
        exact = local + "|" + (outbound ? dst : src) + "|" + (outbound ? dp : sp);
        // Only UDP has a wildcard-bound owner table; TCP attribution stays exact.
        wildcard = protocol == 17 ? LocalKey(protocol, version == 4 ? "0.0.0.0" : "::", outbound ? sp : dp) : "";
        if (protocol == 6) local = "";
        return true;
    }
    private static string ProcessPath(uint pid) {
        if (pid <= 4) return null;
        using (var h = Divert.OpenProcess(0x1000, false, pid)) {
            if (h.IsInvalid) return null;
            var text = new StringBuilder(1024); int length = text.Capacity;
            return Divert.QueryFullProcessImageName(h, 0, text, ref length) ? text.ToString() : null;
        }
    }
    private static void Add(Dictionary<string, string> map, string key, string path) {
        string existing;
        if (map.TryGetValue(key, out existing) && !String.Equals(existing, path, StringComparison.OrdinalIgnoreCase)) map[key] = null;
        else if (!map.ContainsKey(key)) map[key] = path;
    }
    private void RefreshOwners() {
        var next = new Dictionary<string, string>(); var paths = new Dictionary<uint, string>();
        foreach (int family in new int[] { 2, 23 }) foreach (bool tcp in new bool[] { true, false }) {
            uint size = 0;
            if (tcp) Divert.GetExtendedTcpTable(IntPtr.Zero, ref size, false, family, 5, 0);
            else Divert.GetExtendedUdpTable(IntPtr.Zero, ref size, false, family, 1, 0);
            if (size == 0 || size > 16777216) throw new Exception("连接表大小无效");
            var ptr = Marshal.AllocHGlobal((int)size);
            try {
                uint error = tcp ? Divert.GetExtendedTcpTable(ptr, ref size, false, family, 5, 0) : Divert.GetExtendedUdpTable(ptr, ref size, false, family, 1, 0);
                if (error != 0) throw new Win32Exception((int)error);
                var bytes = new byte[size]; Marshal.Copy(ptr, bytes, 0, bytes.Length);
                int count = BitConverter.ToInt32(bytes, 0), stride = family == 2 ? (tcp ? 24 : 12) : (tcp ? 56 : 28);
                if (count < 0 || count > 65536 || 4L + count * (long)stride > bytes.Length) throw new Exception("连接表超出范围");
                for (int i = 0, at = 4; i < count; i++, at += stride) {
                    int addr = at + (family == 2 && tcp ? 4 : 0), port = at + (family == 2 ? (tcp ? 8 : 4) : 20);
                    uint pid = BitConverter.ToUInt32(bytes, at + stride - 4); string path;
                    if (!paths.TryGetValue(pid, out path)) { path = ProcessPath(pid); paths[pid] = path; }
                    string key = LocalKey(tcp ? 6 : 17, Ip(bytes, addr, family == 2 ? 4 : 16), Port(bytes, port));
                    if (tcp) key += "|" + Ip(bytes, at + (family == 2 ? 12 : 24), family == 2 ? 4 : 16) + "|" + Port(bytes, at + (family == 2 ? 16 : 44));
                    Add(next, key, path);
                }
            } finally { Marshal.FreeHGlobal(ptr); }
        }
        lock (ownersGate) { owners = next; recent.Clear(); }
    }
    private void Tables() {
        int failures = 0;
        while (running) {
            Thread.Sleep(250);
            try { RefreshOwners(); failures = 0; }
            catch { lock (ownersGate) { owners.Clear(); recent.Clear(); } if (++failures >= 8) throw; }
        }
    }
    private static string FlowIp(byte[] address, int offset) {
        var words = new uint[4]; for (int i = 0; i < 4; i++) words[i] = BitConverter.ToUInt32(address, offset + i * 4);
        var text = new StringBuilder(64); if (!Divert.WinDivertHelperFormatIPv6Address(words, text, 64)) return "";
        return NormalIp(IPAddress.Parse(text.ToString()));
    }
    private void Flows() {
        var address = new byte[80];
        while (running) {
            uint length; if (!Divert.WinDivertRecv(flow, null, 0, out length, address)) { if (running) throw new Win32Exception(); break; }
            string key = LocalKey(address[72], FlowIp(address, 36), BitConverter.ToUInt16(address, 68)) + "|" + FlowIp(address, 52) + "|" + BitConverter.ToUInt16(address, 70);
            string path = address[9] == 1 ? ProcessPath(BitConverter.ToUInt32(address, 32)) : null;
            lock (ownersGate) {
                if (address[9] == 2) { recent.Remove(key); owners.Remove(key); }
                else if (recent.Count < 65536) Add(recent, key, path);
            }
        }
    }
}
