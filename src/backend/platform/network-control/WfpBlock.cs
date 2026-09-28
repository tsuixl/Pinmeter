using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Linq;
using System.Runtime.InteropServices;

// Native ALE application filters also cover loopback/proxy connections.
// Layouts and constants: Windows SDK fwpmtypes.h, fwptypes.h and fwpmu.h (x64).
internal static class WfpBlock {
    private const uint FilterMissing = 0x80320003, SublayerMissing = 0x80320007;
    private const uint Persistent = 1, Disabled = 32, Block = 0x1001, ByteBlob = 12;
    private static readonly Guid AppId = new Guid("d78e1e87-8644-4ea5-9437-d809ecefc971");
    private static readonly Guid SubKey = new Guid(Program.Hash(Program.Owner + ".Wfp"));
    private static readonly Guid[] Layers = {
        new Guid("c38d57d1-05a7-4c33-904f-7fbceee60e82"), // AUTH_CONNECT_V4
        new Guid("4a72393b-319f-44bc-84c3-ba54dcb3b6b4"), // AUTH_CONNECT_V6
        new Guid("e1cd9fe7-f4b5-4273-96c0-592e487b8650"), // AUTH_RECV_ACCEPT_V4
        new Guid("a3b42c97-9f04-4672-b87e-cee9c483257f")  // AUTH_RECV_ACCEPT_V6
    };
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] private struct Display {
        [MarshalAs(UnmanagedType.LPWStr)] internal string name;
        [MarshalAs(UnmanagedType.LPWStr)] internal string description;
    }
    [StructLayout(LayoutKind.Sequential)] private struct Blob { internal uint size; internal IntPtr data; }
    [StructLayout(LayoutKind.Sequential)] private struct Value { internal uint type; internal IntPtr data; }
    [StructLayout(LayoutKind.Sequential)] private struct Condition { internal Guid field; internal uint match; internal Value value; }
    [StructLayout(LayoutKind.Sequential)] private struct Action { internal uint type; internal Guid key; }
    [StructLayout(LayoutKind.Explicit)] private struct Context {
        [FieldOffset(0)] internal ulong raw;
        [FieldOffset(0)] internal Guid key;
    }
    [StructLayout(LayoutKind.Sequential)] private struct Filter {
        internal Guid key; internal Display display; internal uint flags; internal IntPtr provider;
        internal Blob providerData; internal Guid layer, sublayer; internal Value weight;
        internal uint count; internal IntPtr conditions; internal Action action; internal Context context;
        internal IntPtr reserved; internal ulong id; internal Value effectiveWeight;
    }
    [StructLayout(LayoutKind.Sequential)] private struct Sublayer {
        internal Guid key; internal Display display; internal uint flags; internal IntPtr provider;
        internal Blob providerData; internal ushort weight;
    }
    [StructLayout(LayoutKind.Sequential)] private struct EnumTemplate {
        internal IntPtr provider; internal Guid layer; internal uint type, flags; internal IntPtr context;
        internal uint count; internal IntPtr conditions; internal uint actionMask; internal IntPtr callout;
    }
    [DllImport("fwpuclnt.dll", CharSet = CharSet.Unicode)] private static extern uint FwpmEngineOpen0(string server, uint auth, IntPtr identity, IntPtr session, out IntPtr engine);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmEngineClose0(IntPtr engine);
    [DllImport("fwpuclnt.dll")] private static extern void FwpmFreeMemory0(ref IntPtr memory);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmTransactionBegin0(IntPtr engine, uint flags);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmTransactionCommit0(IntPtr engine);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmTransactionAbort0(IntPtr engine);
    [DllImport("fwpuclnt.dll", CharSet = CharSet.Unicode)] private static extern uint FwpmGetAppIdFromFileName0(string path, out IntPtr id);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmSubLayerGetByKey0(IntPtr engine, ref Guid key, out IntPtr layer);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmSubLayerAdd0(IntPtr engine, ref Sublayer layer, IntPtr security);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmSubLayerDeleteByKey0(IntPtr engine, ref Guid key);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmFilterAdd0(IntPtr engine, ref Filter filter, IntPtr security, out ulong id);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmFilterGetByKey0(IntPtr engine, ref Guid key, out IntPtr filter);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmFilterDeleteByKey0(IntPtr engine, ref Guid key);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmFilterCreateEnumHandle0(IntPtr engine, ref EnumTemplate template, out IntPtr handle);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmFilterEnum0(IntPtr engine, IntPtr handle, uint count, out IntPtr entries, out uint returned);
    [DllImport("fwpuclnt.dll")] private static extern uint FwpmFilterDestroyEnumHandle0(IntPtr engine, IntPtr handle);
    private sealed class Engine : IDisposable {
        internal IntPtr handle;
        internal Engine() { Check(FwpmEngineOpen0(null, 10, IntPtr.Zero, IntPtr.Zero, out handle)); }
        public void Dispose() { if (handle != IntPtr.Zero) FwpmEngineClose0(handle); handle = IntPtr.Zero; }
    }
    private static void Check(uint error) {
        if (error != 0) throw new Exception("应用连接阻断失败 (WFP 0x" + error.ToString("X8") + ")：" + new Win32Exception((int)error).Message);
    }
    private static T Read<T>(IntPtr p) { return (T)Marshal.PtrToStructure(p, typeof(T)); }
    private static string Name(string path, int layer) { return Program.Owner + ".Wfp." + Program.Hash(path.ToLowerInvariant()) + "." + layer; }
    private static Guid Key(string path, int layer) { return new Guid(Program.Hash(Name(path, layer))); }
    private static bool Owned(Filter f, string path, int layer) {
        return f.sublayer == SubKey && f.key == Key(path, layer) && f.layer == Layers[layer]
            && f.display.name == Name(path, layer) && f.display.description == path;
    }
    private static bool Owned(Sublayer layer) {
        // BFE can choose the closest available weight; it is priority, not ownership.
        return layer.key == SubKey && layer.display.name == Program.Owner
            && layer.display.description == "Pinmeter application network blocking"
            && layer.provider == IntPtr.Zero && layer.flags == Persistent;
    }
    private static bool HasSublayer(IntPtr engine) {
        Guid key = SubKey; IntPtr p; uint result = FwpmSubLayerGetByKey0(engine, ref key, out p);
        if (result == SublayerMissing) return false; Check(result);
        try {
            var layer = Read<Sublayer>(p);
            if (!Owned(layer))
                throw new Exception("WFP 子层归属冲突，未修改现有策略（name=" + layer.display.name
                    + ", description=" + layer.display.description + ", flags=" + layer.flags
                    + ", provider=" + layer.provider + ", weight=" + layer.weight + "）");
            return true;
        } finally { FwpmFreeMemory0(ref p); }
    }
    private static IntPtr Get(IntPtr engine, string path, int layer) {
        Guid key = Key(path, layer); IntPtr p; uint result = FwpmFilterGetByKey0(engine, ref key, out p);
        if (result == FilterMissing) return IntPtr.Zero; Check(result);
        if (!Owned(Read<Filter>(p), path, layer)) { FwpmFreeMemory0(ref p); throw new Exception("WFP 规则归属冲突，未修改现有策略"); }
        return p;
    }
    private static byte[] Bytes(IntPtr p) {
        var b = Read<Blob>(p); if (b.size > 8192 || b.data == IntPtr.Zero) throw new Exception("无效的 WFP 应用身份");
        var bytes = new byte[b.size]; Marshal.Copy(b.data, bytes, 0, bytes.Length); return bytes;
    }
    internal sealed class State {
        internal int present, active;
        internal bool inbound, outbound;
    }
    internal static State Inspect(string path) {
        var state = new State();
        using (var engine = new Engine()) {
            if (!HasSublayer(engine.handle)) return state;
            IntPtr app = IntPtr.Zero;
            try {
                for (int i = 0; i < Layers.Length; i++) {
                    IntPtr p = Get(engine.handle, path, i); if (p == IntPtr.Zero) continue;
                    try {
                        state.present++; if (i < 2) state.outbound = true; else state.inbound = true;
                        var f = Read<Filter>(p);
                        if ((f.flags & (Persistent | Disabled)) != Persistent || f.provider != IntPtr.Zero
                            || f.action.type != Block || f.count != 1 || f.conditions == IntPtr.Zero) continue;
                        var c = Read<Condition>(f.conditions);
                        if (c.field != AppId || c.match != 0 || c.value.type != ByteBlob || c.value.data == IntPtr.Zero) continue;
                        if (app == IntPtr.Zero) Check(FwpmGetAppIdFromFileName0(path, out app));
                        if (Bytes(c.value.data).SequenceEqual(Bytes(app))) state.active++;
                    } finally { FwpmFreeMemory0(ref p); }
                }
            } finally { if (app != IntPtr.Zero) FwpmFreeMemory0(ref app); }
        }
        return state;
    }
    internal static void Apply(string path, bool blocked) {
        Diagnostics.Record("wfp-apply", new { path, blocked });
        using (var engine = new Engine()) {
            IntPtr app = IntPtr.Zero, conditions = IntPtr.Zero;
            Check(FwpmTransactionBegin0(engine.handle, 0)); bool committed = false;
            try {
                if (blocked) {
                    if (!HasSublayer(engine.handle)) {
                        // No provider/service dependency: persistence must not depend on a Pinmeter service.
                        var layer = new Sublayer { key = SubKey, flags = Persistent, weight = 65500,
                            display = new Display { name = Program.Owner, description = "Pinmeter application network blocking" } };
                        Check(FwpmSubLayerAdd0(engine.handle, ref layer, IntPtr.Zero));
                    }
                    Check(FwpmGetAppIdFromFileName0(path, out app));
                    var condition = new Condition { field = AppId, value = new Value { type = ByteBlob, data = app } };
                    conditions = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(Condition))); Marshal.StructureToPtr(condition, conditions, false);
                }
                for (int i = 0; i < Layers.Length; i++) {
                    IntPtr existing = Get(engine.handle, path, i);
                    if (existing != IntPtr.Zero) { FwpmFreeMemory0(ref existing); Guid key = Key(path, i); Check(FwpmFilterDeleteByKey0(engine.handle, ref key)); }
                    if (!blocked) continue;
                    var filter = new Filter { key = Key(path, i), display = new Display { name = Name(path, i), description = path },
                        flags = Persistent, layer = Layers[i], sublayer = SubKey, count = 1, conditions = conditions,
                        weight = new Value { type = 1, data = new IntPtr(15) }, action = new Action { type = Block } };
                    ulong id; Check(FwpmFilterAdd0(engine.handle, ref filter, IntPtr.Zero, out id));
                }
                Check(FwpmTransactionCommit0(engine.handle)); committed = true;
                Diagnostics.Record("wfp-apply-committed", new { path, blocked });
            } finally {
                if (!committed) FwpmTransactionAbort0(engine.handle);
                if (conditions != IntPtr.Zero) Marshal.FreeHGlobal(conditions);
                if (app != IntPtr.Zero) FwpmFreeMemory0(ref app);
            }
        }
    }
    private static List<Guid> OwnedFilters(IntPtr engine, List<object> details = null) {
        var owned = new List<Guid>(); if (!HasSublayer(engine)) return owned;
        for (int layer = 0; layer < Layers.Length; layer++) {
            var template = new EnumTemplate { layer = Layers[layer], flags = 0x10, actionMask = UInt32.MaxValue };
            IntPtr handle; Check(FwpmFilterCreateEnumHandle0(engine, ref template, out handle));
            try {
                uint total = 0;
                while (true) {
                    IntPtr entries; uint count; Check(FwpmFilterEnum0(engine, handle, 128, out entries, out count));
                    try {
                        if ((total += count) > 65536) throw new Exception("WFP 枚举超出安全范围");
                        for (int i = 0; i < count; i++) {
                            var f = Read<Filter>(Marshal.ReadIntPtr(entries, i * IntPtr.Size));
                            if (f.sublayer != SubKey) continue;
                            if (String.IsNullOrEmpty(f.display.description) || !Owned(f, f.display.description, layer)) throw new Exception("本安装 WFP 子层存在归属异常规则，未删除");
                            owned.Add(f.key);
                            if (details != null) details.Add(new { key = f.key.ToString(), path = f.display.description,
                                name = f.display.name, layer = f.layer.ToString(), flags = f.flags, action = f.action.type });
                        }
                    } finally { if (entries != IntPtr.Zero) FwpmFreeMemory0(ref entries); }
                    if (count == 0) break;
                }
            } finally { FwpmFilterDestroyEnumHandle0(engine, handle); }
        }
        return owned;
    }
    internal static int CountOwned() { using (var engine = new Engine()) return OwnedFilters(engine.handle).Count; }
    internal static List<object> InspectOwned() {
        using (var engine = new Engine()) {
            var details = new List<object>(); OwnedFilters(engine.handle, details); return details;
        }
    }
    internal static void Cleanup() {
        using (var engine = new Engine()) {
            Check(FwpmTransactionBegin0(engine.handle, 0)); bool committed = false;
            try {
                var details = new List<object>(); var filters = OwnedFilters(engine.handle, details);
                Diagnostics.Record("wfp-cleanup-targets", new { rules = details });
                foreach (var item in filters) { Guid key = item; Check(FwpmFilterDeleteByKey0(engine.handle, ref key)); }
                if (HasSublayer(engine.handle)) { Guid key = SubKey; Check(FwpmSubLayerDeleteByKey0(engine.handle, ref key)); }
                Check(FwpmTransactionCommit0(engine.handle)); committed = true;
                Diagnostics.Record("wfp-cleanup-committed", new { deleted = filters.Count });
            } finally { if (!committed) FwpmTransactionAbort0(engine.handle); }
        }
    }
    internal static int CleanupRequest() {
        if (Program.Admin) return Program.Cleanup();
        // Fixed executable/argument; return the elevated cleanup's verified exit code to NSIS.
        try {
            using (var p = Process.Start(new ProcessStartInfo(typeof(Program).Assembly.Location, "--cleanup") {
                UseShellExecute = true, Verb = "runas", WindowStyle = ProcessWindowStyle.Hidden })) {
                p.WaitForExit(); return p.ExitCode;
            }
        } catch { return 1; }
    }
    internal static void SelfTest() {
        if (IntPtr.Size != 8 || Marshal.SizeOf(typeof(Filter)) != 200 || Marshal.SizeOf(typeof(Condition)) != 40
            || Marshal.SizeOf(typeof(Sublayer)) != 72 || Marshal.OffsetOf(typeof(Filter), "context").ToInt32() != 152)
            throw new Exception("WFP x64 ABI layout mismatch");
        if (Key(@"C:\Apps\Browser.exe", 0) != Key(@"c:\apps\browser.exe", 0) || Key(@"C:\Apps\Browser.exe", 0) == Key(@"C:\Apps\Browser.exe", 1))
            throw new Exception("WFP rule identity regression");
        var layer = new Sublayer { key = SubKey, flags = Persistent, weight = 65499,
            display = new Display { name = Program.Owner, description = "Pinmeter application network blocking" } };
        if (!Owned(layer)) throw new Exception("BFE-assigned weight must not change sublayer ownership");
        layer.key = Guid.Empty;
        if (Owned(layer)) throw new Exception("Foreign sublayer identity accepted");
        layer.key = SubKey; layer.display.name = "other";
        if (Owned(layer)) throw new Exception("Foreign sublayer name accepted");
        layer.display.name = Program.Owner; layer.provider = new IntPtr(1);
        if (Owned(layer)) throw new Exception("Foreign sublayer provider accepted");
    }
}
