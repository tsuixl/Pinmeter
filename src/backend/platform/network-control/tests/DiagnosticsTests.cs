using System;
using System.IO;
using System.Linq;
using System.Threading;
using System.Web.Script.Serialization;
using System.Collections.Generic;

// Compiled separately by tools/check-network-diagnostics.mjs; never packaged.
internal static class DiagnosticsTests {
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer { MaxJsonLength = 262144 };
    private static Dictionary<string, object> Status() { return Json.Deserialize<Dictionary<string, object>>(Json.Serialize(Diagnostics.Status())); }
    private static void Check(bool value, string detail) { if (!value) throw new Exception(detail); }
    private static int Main(string[] args) {
        try {
            string mode = args[0], directory = args[1];
            Directory.CreateDirectory(directory);
            if (mode == "unwritable") {
                string blocked = Path.Combine(directory, "file"); File.WriteAllText(blocked, "occupied");
                Diagnostics.Start(blocked);
                Check((string)Status()["error"] != "", "Write failure was hidden");
                Diagnostics.Record("still-running", new { ok = true }); Diagnostics.Finish("test");
            } else if (mode == "retention") {
                for (int i = 0; i < 15; i++) {
                    string old = Path.Combine(directory, "control-old-" + i + ".jsonl");
                    File.WriteAllText(old, "{}"); File.SetLastWriteTimeUtc(old, DateTime.UtcNow.AddDays(-2).AddMinutes(i));
                }
                string live = Path.Combine(directory, "control-live.jsonl");
                File.WriteAllText(live, "{}"); File.SetLastWriteTimeUtc(live, DateTime.UtcNow.AddDays(-3));
                using (var held = new FileStream(live, FileMode.Open, FileAccess.Write, FileShare.Read)) {
                    Diagnostics.Start(directory); Diagnostics.Finish("test");
                    Check(File.Exists(live), "Deleted a live helper log");
                    Check(Directory.GetFiles(directory, "*.jsonl").Length <= 12, "History was not bounded");
                }
            } else {
                Diagnostics.Start(directory);
                Check((string)Status()["error"] == "", "Log startup failed");
                string path = (string)Status()["path"];
                FileStream blocker = null;
                if (mode == "write-failure") blocker = new FileStream(path + ".previous.jsonl", FileMode.CreateNew, FileAccess.Write, FileShare.Read);
                try {
                    var data = new { message = new string('x', 16000) };
                    int count = mode == "queue" ? 100000 : 200;
                    for (int i = 0; i < count; i++) {
                        Diagnostics.Record("test-entry", data);
                        if (mode != "queue") Thread.Sleep(3);
                    }
                    if (mode == "rotation") Check(Diagnostics.FlushBeforeRelease(), "Cleanup checkpoint was not written");
                    Diagnostics.Finish("test");
                    var state = Status();
                    if (mode == "write-failure") Check((string)state["error"] != "", "Rollover I/O error was hidden");
                    else {
                        Check((string)state["error"] == "", "Unexpected writer failure");
                        if (mode == "queue") Check(Convert.ToInt64(state["dropped"]) > 0, "Queue did not bound overload");
                        var files = Directory.GetFiles(directory, "*.jsonl");
                        Check(files.Length == 2, "Expected current and previous log");
                        foreach (string item in files) {
                            Check(new FileInfo(item).Length <= 1048576, "File exceeded limit");
                            foreach (string line in File.ReadLines(item)) Json.DeserializeObject(line);
                        }
                    }
                } finally { if (blocker != null) blocker.Dispose(); }
            }
            Console.WriteLine("PASS: " + mode); return 0;
        } catch (Exception e) { Console.Error.WriteLine(e); return 1; }
    }
}
