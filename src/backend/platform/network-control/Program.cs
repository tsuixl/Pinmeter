// Windows adapter: bounded stdio protocol, firewall rules and optional packet pacing.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Security.Principal;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

public sealed class Rule {
    public string id, name, path;
    public uint? download, upload;
    public bool blocked;
}
public sealed class Request { public string command; public Rule[] rules; }
internal static class Program {
    internal static readonly string Root = Path.GetFullPath(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, ".."));
    internal static readonly string Owner = "Pinmeter.NetworkControl." + Hash(Root.ToLowerInvariant());
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer { MaxJsonLength = 262144, RecursionLimit = 16 };
    private static Rule[] Rules = new Rule[0];
    private static PacketEngine Engine;
    private static string EngineError = "";
    private static object EngineState() {
        var current = Engine;
        return new { engine_present = current != null, engine = current == null ? null : current.DiagnosticState(),
            rule_count = Rules.Length, error = EngineError };
    }
    internal static bool Admin { get { return new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator); } }
    [DllImport("ntdll.dll")] private static extern int NtQueryInformationProcess(IntPtr process, int type, IntPtr info, int length, out int returned);
    internal static string Hash(string value) {
        using (var sha = SHA256.Create()) return BitConverter.ToString(sha.ComputeHash(Encoding.UTF8.GetBytes(value))).Replace("-", "").Substring(0, 32);
    }
    private static bool ParentIs(int pid) {
        var buffer = Marshal.AllocHGlobal(48);
        try { int size; return NtQueryInformationProcess(Process.GetCurrentProcess().Handle, 0, buffer, 48, out size) == 0 && Marshal.ReadIntPtr(buffer, 40).ToInt64() == pid; }
        finally { Marshal.FreeHGlobal(buffer); }
    }
    internal static void Validate(Rule r) {
        if (r == null || String.IsNullOrEmpty(r.path) || String.IsNullOrEmpty(r.id) || r.path.Length > 1024 || r.id.Length > 1024) throw new Exception("无效的应用路径");
        var full = Path.GetFullPath(r.path);
        if (full.StartsWith(@"\\") || !String.Equals(full, r.path, StringComparison.OrdinalIgnoreCase) || !full.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)
            || full.StartsWith(Root.TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase)
            || full.StartsWith(Environment.GetFolderPath(Environment.SpecialFolder.Windows).TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase))
            throw new Exception("此程序不支持网络控制（系统或 Pinmeter 组件）");
        foreach (var rate in new uint?[] { r.download, r.upload }) if (rate.HasValue && (rate.Value < 16000 || rate.Value > 1000000000)) throw new Exception("限速值超出范围");
    }
    private static string ReadLine() {
        var s = new StringBuilder();
        for (int i = 0; i < 262144; i++) { int c = Console.Read(); if (c < 0) return null; if (c == '\n') return s.ToString(); s.Append((char)c); }
        throw new Exception("控制消息过大");
    }
    private static void Apply(Rule[] rules) {
        if (!Admin) throw new Exception("需要启动时取得的管理员权限");
        if (rules == null || rules.Length > 128) throw new Exception("规则数量无效");
        var ids = new HashSet<string>(); var paths = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var r in rules) { Validate(r); if (!ids.Add(r.id) || !paths.Add(r.path)) throw new Exception("重复规则"); }
        Rules = rules;
        Diagnostics.Record("apply", new { rules });
        EngineError = "";
        // Updating rules cancels old queues before installing firewall blocks.
        if (Engine != null) {
            try { Engine.Update(rules); }
            catch (Exception e) { Engine.Dispose(); Engine = null; EngineError = e.Message; }
        }
        bool needed = rules.Any(r => !r.blocked && (r.download.HasValue || r.upload.HasValue));
        Diagnostics.Record("engine-decision", new { needed, engine_present = Engine != null });
        if (needed && (Engine == null || !Engine.Healthy)) {
            if (Engine != null) Engine.Dispose(); Engine = null;
            try { Engine = new PacketEngine(rules); } catch (Exception e) { EngineError = "限速不可用：" + e.Message; }
        }
        if (!needed && Engine != null) { Engine.Dispose(); Engine = null; }
        foreach (var r in rules) {
            string error = "";
            try { WfpBlock.Apply(r.path, r.blocked); } catch (Exception e) { error = e.Message; }
            try { Firewall.Apply(r); } catch (Exception e) { error += e.Message; }
            if (error != "") { Firewall.Errors[r.id] = error; Diagnostics.Record("rule-error", new { r.id, r.path, error }); }
        }
        Diagnostics.Record("apply-complete", EngineState());
    }
    private static object Snapshot() {
        var rows = new List<object>();
        string firewallError = Firewall.Capability();
        foreach (var r in Rules) {
            bool inbound = false, outbound = false; string error = "";
            try { inbound = Firewall.Blocked(r.path, 1); outbound = Firewall.Blocked(r.path, 2); }
            catch (Exception e) { error = e.Message; }
            bool firewallOk = r.blocked ? inbound && outbound && firewallError == "" : !inbound && !outbound;
            bool wfpOk = false;
            try {
                var wfp = WfpBlock.Inspect(r.path);
                wfpOk = r.blocked ? wfp.active == 4 : wfp.present == 0;
                inbound |= wfp.inbound; outbound |= wfp.outbound;
            } catch (Exception e) { error += e.Message; }
            string applyError; if (Firewall.Errors.TryGetValue(r.id, out applyError)) error += applyError;
            bool needsLimit = r.download.HasValue || r.upload.HasValue;
            bool limiting = !r.blocked && needsLimit && Engine != null && Engine.Healthy;
            bool ok = error == "" && firewallOk && wfpOk && (r.blocked || !needsLimit || limiting);
            string detail = error;
            if (r.blocked && firewallError != "") detail += firewallError;
            if (!firewallOk && detail == "") detail = "系统禁用规则与设置不一致，请重试";
            if (!wfpOk && detail == "") detail = "应用连接阻断未确认（包括本机代理），请重试";
            if (!r.blocked && needsLimit && !limiting) detail += EngineError != "" ? EngineError : "限速执行器已停止";
            if (!File.Exists(r.path)) detail += " 程序路径已不存在，可清除规则";
            if (ok && detail == "") detail = r.blocked ? "已阻止该程序入站、出站及本机代理连接" : limiting ? "限速已启用；代理、回环及无法归属的流量不在保证范围内" : "已解除 Pinmeter 的限制";
            rows.Add(new { id = r.id, status = ok ? "applied" : inbound || outbound || limiting ? "partial" : "failed", detail,
                inbound_blocked = inbound, outbound_blocked = outbound, limiting });
        }
        return new { available = Admin, detail = !Admin ? "启动时未取得管理员权限" : firewallError,
            driver_available = PacketEngine.FilesPresent, firewall_available = Admin && firewallError == "", rules = rows,
            diagnostics = Diagnostics.Status(), engine = EngineState() };
    }
    private static int Main(string[] args) {
        Console.InputEncoding = Encoding.UTF8; Console.OutputEncoding = new UTF8Encoding(false);
        if (args.Length == 1 && args[0] == "--self-test") {
            try { PacketEngine.SelfTest(); WfpBlock.SelfTest(); CleanupSelfTest(); Uninstall.SelfTest(); Console.WriteLine("PASS: pacing debt, IPv4/IPv6 packet attribution, protected paths, WFP x64 layout and identity, independent cleanup and uninstall sequencing"); return 0; }
            catch (Exception e) { Console.Error.WriteLine(e.Message); return 1; }
        }
        if (args.Length == 1 && args[0] == "--verify-clean") {
            if (!Admin) return 5;
            try { return Firewall.Owned().Count == 0 && WfpBlock.CountOwned() == 0 ? 0 : 1; } catch { return 2; }
        }
        if (args.Length == 1 && args[0] == "--inspect-platform") {
            try {
                string missing = Path.Combine(Path.GetTempPath(), "pinmeter-missing-network-probe.exe");
                Console.WriteLine(Json.Serialize(new { admin = Admin, driver_files = PacketEngine.FilesPresent,
                    firewall_detail = Firewall.Capability(), missing_inbound = Firewall.Blocked(missing, 1),
                    missing_outbound = Firewall.Blocked(missing, 2), owned_rules = Firewall.Owned().Count,
                    wfp_owned_rules = Admin ? WfpBlock.CountOwned() : -1 }));
                return 0;
            } catch (Exception e) { Console.Error.WriteLine(e); return 1; }
        }
        if (args.Length == 1 && args[0] == "--cleanup") {
            Diagnostics.Start();
            try { return Cleanup(); } finally { Diagnostics.Finish("cleanup-command"); }
        }
        if (args.Length == 1 && args[0] == "--cleanup-request") return WfpBlock.CleanupRequest();
        if (args.Length == 1 && args[0] == "--uninstall-cleanup-request") return WfpBlock.CleanupRequest(true);
        if (args.Length == 1 && args[0] == "--uninstall-cleanup") {
            Diagnostics.Start();
            try { return Uninstall.Cleanup(Cleanup, Uninstall.RemoveStartup); }
            finally { Diagnostics.Finish("uninstall-cleanup"); }
        }
        int pid;
        if (args.Length != 1 || !Int32.TryParse(args[0], out pid) || !ParentIs(pid)) return 2;
        Diagnostics.Start();
        Diagnostics.StartSampling(EngineState);
        using (var parent = Process.GetProcessById(pid)) {
            new Thread(() => { parent.WaitForExit(); Diagnostics.Finish("parent-exited"); Environment.Exit(0); }) { IsBackground = true }.Start();
            try {
                string line;
                while ((line = ReadLine()) != null) {
                    try {
                        var request = Json.Deserialize<Request>(line);
                        if (request.command != "inspect") Diagnostics.Record("command", new { command = request.command });
                        if (request.command == "shutdown") break;
                        if (request.command == "apply") Apply(request.rules);
                        else if (request.command == "release-all") {
                            if (request.rules == null || request.rules.Any(r => r == null || r.blocked || r.download.HasValue || r.upload.HasValue))
                                throw new Exception("解除请求不能包含有效限制");
                            Apply(request.rules);
                            if (Cleanup() != 0) throw new Exception("未能清除全部网络限制，请重试；未退出 Pinmeter");
                            Firewall.Errors.Clear();
                        }
                        else if (request.command != "inspect") throw new Exception("未知命令");
                        Console.WriteLine(Json.Serialize(new { ok = true, report = Snapshot() }));
                    } catch (Exception e) {
                        Diagnostics.Record("command-error", new { error = e.ToString() });
                        Console.WriteLine(Json.Serialize(new { ok = false, error = e.Message }));
                    }
                    Console.Out.Flush();
                }
            } finally {
                if (Engine != null) Engine.Dispose();
                Diagnostics.Record("shutdown-state", EngineState());
                Diagnostics.Finish("shutdown-or-eof");
            }
        }
        return 0;
    }
    internal static int Cleanup() {
        Diagnostics.Record("cleanup-begin", new { admin = Admin });
        if (!Admin) { Diagnostics.Record("cleanup-error", new { error = "permission_denied" }); return 5; }
        try {
            var result = CleanupSystems(WfpBlock.Cleanup, Firewall.Cleanup, WfpBlock.CountOwned, () => Firewall.Owned().Count);
            Diagnostics.Record("cleanup-complete", new { wfp = result.wfp, firewall = result.firewall,
                verified = result.Verified, errors = result.errors });
            if (!result.Verified) Diagnostics.Record("cleanup-error", new { errors = result.errors, result.wfp, result.firewall });
            return result.Verified ? 0 : 1;
        } catch (Exception e) { Diagnostics.Record("cleanup-error", new { error = e.ToString() }); return 1; }
        // The host may terminate this helper immediately after the successful reply.
        // Bound the diagnostic wait; logging must never prevent network recovery.
        finally { Diagnostics.FlushBeforeRelease(); }
    }
    internal sealed class CleanupResult {
        internal int? wfp, firewall;
        internal readonly List<string> errors = new List<string>();
        internal bool Verified { get { return errors.Count == 0 && wfp == 0 && firewall == 0; } }
    }
    // Each backend is independently recoverable. A failure must not prevent the
    // other cleanup or either readback; unknown counts must never look like zero.
    internal static CleanupResult CleanupSystems(Action clearWfp, Action clearFirewall, Func<int> countWfp, Func<int> countFirewall) {
        var result = new CleanupResult();
        try { clearWfp(); } catch (Exception e) { result.errors.Add("WFP 清理：" + e.Message); }
        try { clearFirewall(); } catch (Exception e) { result.errors.Add("防火墙清理：" + e.Message); }
        try { result.wfp = countWfp(); } catch (Exception e) { result.errors.Add("WFP 核对：" + e.Message); }
        try { result.firewall = countFirewall(); } catch (Exception e) { result.errors.Add("防火墙核对：" + e.Message); }
        return result;
    }
    private static void CleanupSelfTest() {
        // Exercise every combination of cleanup/readback failures without OS writes.
        for (int failures = 0; failures < 16; failures++) {
            var calls = new List<int>();
            Action<int> step = i => { calls.Add(i); if ((failures & (1 << i)) != 0) throw new Exception("injected " + i); };
            var result = CleanupSystems(() => step(0), () => step(1), () => { step(2); return 0; }, () => { step(3); return 0; });
            if (!calls.SequenceEqual(new[] { 0, 1, 2, 3 }) || result.Verified != (failures == 0)
                || result.wfp.HasValue != ((failures & 4) == 0) || result.firewall.HasValue != ((failures & 8) == 0)
                || result.errors.Count != Enumerable.Range(0, 4).Count(i => (failures & (1 << i)) != 0))
                throw new Exception("Independent cleanup/readback regression: " + failures);
        }
        foreach (int residual in new[] { 1, 2 }) {
            var result = CleanupSystems(() => {}, () => {}, () => residual == 1 ? 1 : 0, () => residual == 2 ? 1 : 0);
            if (result.Verified) throw new Exception("Residual rules reported as released");
        }
    }
}

internal static class Firewall {
    [ComImport, Guid("98325047-C671-4174-8D81-DEFCD3F03186"), InterfaceType(ComInterfaceType.InterfaceIsIDispatch)]
    private interface PolicyView {
        [DispId(7)] object Rules { [return: MarshalAs(UnmanagedType.Interface)] get; }
    }
    // Use fixed IDispatch members for enumeration: dynamic COM binding retains
    // per-object type information across the recurring inventory of every rule.
    // Windows SDK netfw.idl / INetFwRule. Only properties read here are exposed.
    [ComImport, Guid("AF230D27-BABA-4E42-ACED-F524F22CFCE2"), InterfaceType(ComInterfaceType.InterfaceIsIDispatch)]
    private interface RuleView {
        [DispId(1)] string Name { [return: MarshalAs(UnmanagedType.BStr)] get; }
        [DispId(2)] string Description { [return: MarshalAs(UnmanagedType.BStr)] get; }
        [DispId(3)] string ApplicationName { [return: MarshalAs(UnmanagedType.BStr)] get; }
        [DispId(11)] int Direction { get; }
        [DispId(14)] bool Enabled { [return: MarshalAs(UnmanagedType.VariantBool)] get; }
        [DispId(16)] int Profiles { get; }
        [DispId(18)] int Action { get; }
    }
    internal static readonly Dictionary<string, string> Errors = new Dictionary<string, string>();
    private static dynamic Policy() { return Activator.CreateInstance(Type.GetTypeFromProgID("HNetCfg.FwPolicy2", true)); }
    private static void Release(object obj) { if (obj != null && Marshal.IsComObject(obj)) Marshal.FinalReleaseComObject(obj); }
    private static string Name(string path, int direction) { return Program.Owner + "." + Program.Hash(path.ToLowerInvariant()) + "." + direction; }
    private static dynamic Find(dynamic policy, string path, int direction) {
        dynamic rules = policy.Rules;
        try {
            dynamic rule;
            try { rule = rules.Item(Name(path, direction)); }
            catch (Exception e) { if ((uint)Marshal.GetHRForException(e) == 0x80070002) return null; throw; }
            if ((string)rule.Description != Program.Owner || !String.Equals((string)rule.ApplicationName, path, StringComparison.OrdinalIgnoreCase) || (int)rule.Direction != direction) {
                Release(rule); throw new Exception("规则归属冲突，未修改现有规则");
            }
            return rule;
        } finally { Release(rules); }
    }
    internal static string Capability() {
        dynamic policy = null;
        try {
            policy = Policy(); int profiles = policy.CurrentProfileTypes;
            if ((int)policy.LocalPolicyModifyState != 0) return "系统策略禁止本地防火墙规则生效";
            foreach (int p in new int[] { 1, 2, 4 }) if ((profiles & p) != 0 && !(bool)policy.FirewallEnabled[p]) return "当前网络配置的系统防火墙未开启";
            return "";
        } catch (Exception e) { return "防火墙不可用：" + e.Message; }
        finally { Release(policy); }
    }
    internal static bool Blocked(string path, int direction) {
        dynamic policy = Policy(); dynamic rule = null;
        try {
            rule = Find(policy, path, direction);
            return rule != null && (bool)rule.Enabled && (int)rule.Action == 0 && ((int)rule.Profiles & 7) == 7 && (int)rule.Protocol == 256
                && (string)rule.LocalAddresses == "*" && (string)rule.RemoteAddresses == "*" && (string)rule.InterfaceTypes == "All";
        } finally { Release(rule); Release(policy); }
    }
    private static void Set(string path, int direction, bool blocked) {
        dynamic policy = Policy(); dynamic rule = null; dynamic rules = null;
        try {
            rule = Find(policy, path, direction); rules = policy.Rules;
            if (!blocked) { if (rule != null) rules.Remove(Name(path, direction)); return; }
            bool fresh = rule == null;
            if (fresh) rule = Activator.CreateInstance(Type.GetTypeFromProgID("HNetCfg.FWRule", true));
            rule.Name = Name(path, direction); rule.Description = Program.Owner;
            rule.ApplicationName = path; rule.Protocol = 256; rule.Direction = direction;
            rule.Profiles = Int32.MaxValue; rule.Action = 0; rule.LocalAddresses = "*"; rule.RemoteAddresses = "*";
            rule.InterfaceTypes = "All"; rule.Enabled = true;
            if (fresh) rules.Add(rule);
        } finally { Release(rule); Release(rules); Release(policy); }
    }
    internal static void Apply(Rule r) {
        Diagnostics.Record("firewall-apply", new { r.path, r.blocked });
        Errors.Remove(r.id);
        bool beforeIn = Blocked(r.path, 1), beforeOut = Blocked(r.path, 2);
        // An unavailable firewall must not prevent removal of our old blocks.
        if (r.blocked) { string error = Capability(); if (error != "") throw new Exception(error); }
        try { Set(r.path, 1, r.blocked); Set(r.path, 2, r.blocked); }
        catch (Exception original) {
            try { Set(r.path, 1, beforeIn); Set(r.path, 2, beforeOut); }
            catch (Exception rollback) { throw new Exception(original.Message + "；恢复原规则也失败：" + rollback.Message); }
            throw;
        }
    }
    internal static List<object> InspectOwned() {
        var details = new List<object>(); Owned(details); return details;
    }
    internal static List<string> Owned(List<object> details = null) {
        object policy = Policy(), rules = null;
        var remove = new List<string>();
        try {
            rules = ((PolicyView)policy).Rules;
            foreach (object item in (System.Collections.IEnumerable)rules) {
                try {
                    var rule = (RuleView)item;
                    if (rule.Description != Program.Owner) continue;
                    int direction = rule.Direction; string path = rule.ApplicationName;
                    if ((direction == 1 || direction == 2) && rule.Name == Name(path, direction)) {
                        remove.Add(rule.Name);
                        if (details != null) {
                            try { details.Add(new { name = rule.Name, path, direction,
                                enabled = rule.Enabled, action = rule.Action, profiles = rule.Profiles }); }
                            catch (Exception e) { details.Add(new { path, direction, error = e.Message }); }
                        }
                    }
                } finally { Release(item); }
            }
            return remove;
        } finally { Release(rules); Release(policy); }
    }
    internal static void Cleanup() {
        var details = new List<object>(); var remove = Owned(details); dynamic policy = Policy(); dynamic rules = policy.Rules;
        Diagnostics.Record("firewall-cleanup-targets", new { rules = details });
        try { foreach (string name in remove) { rules.Remove(name); Diagnostics.Record("firewall-deleted", new { name }); } }
        finally { Release(rules); Release(policy); }
    }
}
