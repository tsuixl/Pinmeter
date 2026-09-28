using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

// Local evidence only. No packet data, network probes or changes to control decisions.
internal static class Diagnostics {
    private const long MaxBytes = 1048576;
    private static readonly BlockingCollection<object> Pending = new BlockingCollection<object>(256);
    private static readonly ManualResetEvent Stopping = new ManualResetEvent(false);
    private static Thread writer, sampler;
    private static FileStream stream;
    private static string file;
    private static volatile string failure = "";
    private static long dropped;
    private static int finished;
    private sealed class Checkpoint { internal volatile bool written; }
    internal static object Status() { return new { path = file, error = failure, dropped = Interlocked.Read(ref dropped) }; }
    internal static void Start(string testDirectory = null) {
        try {
            string directory = testDirectory ?? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "io.pinmeter.desktop", "diagnostics", "network-control");
            Directory.CreateDirectory(directory);
            // Other live helpers hold their files without delete sharing. Never remove them.
            foreach (var old in new DirectoryInfo(directory).GetFiles("control-*.jsonl")
                .OrderByDescending(f => f.LastWriteTimeUtc).Skip(10)) {
                try { old.Delete(); } catch { }
            }
            file = Path.Combine(directory, "control-" + DateTime.UtcNow.ToString("yyyyMMddTHHmmssfff")
                + "-" + Process.GetCurrentProcess().Id + "-" + Guid.NewGuid().ToString("N") + ".jsonl");
            stream = new FileStream(file, FileMode.CreateNew, FileAccess.Write, FileShare.Read);
            writer = new Thread(WriteLoop) { IsBackground = true, Name = "network-control-log" };
            writer.Start();
            Record("helper-start", new { pid = Process.GetCurrentProcess().Id, owner = Program.Owner,
                root = Program.Root, admin = Program.Admin, protocol = 1 });
        } catch (Exception e) { Failed(e); }
    }
    private static void Failed(Exception e) {
        failure = e.GetType().Name + ": " + e.Message;
        try { Console.Error.WriteLine("Network control diagnostics unavailable: " + failure); } catch { }
    }
    internal static void Record(string kind, object data) {
        if (writer == null || failure != "") return;
        try {
            if (!Pending.TryAdd(new { utc = DateTime.UtcNow.ToString("o"), kind, data })) Interlocked.Increment(ref dropped);
        } catch (InvalidOperationException) { Interlocked.Increment(ref dropped); }
    }
    private static void WriteLoop() {
        var json = new JavaScriptSerializer { MaxJsonLength = 262144, RecursionLimit = 24 };
        try {
            foreach (var entry in Pending.GetConsumingEnumerable()) {
                var checkpoint = entry as Checkpoint;
                if (checkpoint != null) { checkpoint.written = true; continue; }
                byte[] bytes;
                try { bytes = Encoding.UTF8.GetBytes(json.Serialize(entry) + "\n"); }
                catch { Interlocked.Increment(ref dropped); continue; }
                if (stream.Length + bytes.Length > MaxBytes) {
                    stream.Dispose();
                    string previous = file + ".previous.jsonl";
                    if (File.Exists(previous)) File.Delete(previous);
                    File.Move(file, previous);
                    stream = new FileStream(file, FileMode.CreateNew, FileAccess.Write, FileShare.Read);
                }
                stream.Write(bytes, 0, bytes.Length);
                stream.Flush();
            }
        } catch (Exception e) { Failed(e); }
        finally { try { if (stream != null) stream.Dispose(); } catch (Exception e) { Failed(e); } }
    }
    internal static bool FlushBeforeRelease() {
        if (writer == null || failure != "" || Pending.IsAddingCompleted) return false;
        var checkpoint = new Checkpoint();
        try { if (!Pending.TryAdd(checkpoint)) return false; }
        catch (InvalidOperationException) { return false; }
        var elapsed = Stopwatch.StartNew();
        while (!checkpoint.written && failure == "" && elapsed.ElapsedMilliseconds < 250) Thread.Sleep(1);
        return checkpoint.written;
    }
    internal static void StartSampling(Func<object> engine) {
        if (writer == null) return;
        sampler = new Thread(() => {
            do {
                Record("engine-sample", engine());
                try { Record("firewall-inventory", new { rules = Firewall.InspectOwned() }); }
                catch (Exception e) { Record("firewall-inventory-error", new { error = e.Message }); }
                try { Record("wfp-inventory", new { rules = WfpBlock.InspectOwned() }); }
                catch (Exception e) { Record("wfp-inventory-error", new { error = e.Message }); }
            } while (!Stopping.WaitOne(15000));
        }) { IsBackground = true, Name = "network-control-inspect" };
        sampler.Start();
    }
    internal static void Finish(string reason) {
        if (Interlocked.Exchange(ref finished, 1) != 0) return;
        Stopping.Set();
        Record("helper-stop", new { reason, dropped = Interlocked.Read(ref dropped) });
        Pending.CompleteAdding();
        // OS enumeration can stall; never wait for it during release or exit.
        if (writer != null && writer != Thread.CurrentThread) writer.Join(1000);
    }
}
