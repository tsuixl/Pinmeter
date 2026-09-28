using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.Web.Script.Serialization;
using LibreHardwareMonitor.Hardware;

internal static class GpuProgram
{
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer();
    private static readonly Dictionary<string, ISensor> Known = new Dictionary<string, ISensor>();
    [DllImport("cfgmgr32.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern uint CM_Get_Device_ID_List_SizeW(out uint length, string filter, uint flags);
    [DllImport("cfgmgr32.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern uint CM_Get_Device_ID_ListW(string filter, [Out] char[] buffer, uint length, uint flags);

    // Compare present display device identities without resetting LHM's engine counters.
    private static string DisplayDevices()
    {
        const string displayClass = "{4d36e968-e325-11ce-bfc1-08002be10318}";
        const uint flags = 0x200 | 0x100; // FILTER_CLASS | FILTER_PRESENT
        for (int attempt = 0; attempt < 2; attempt++) {
            uint length;
            if (CM_Get_Device_ID_List_SizeW(out length, displayClass, flags) != 0 || length == 0 || length > 65536)
                return null;
            var buffer = new char[length];
            uint result = CM_Get_Device_ID_ListW(displayClass, buffer, length, flags);
            if (result == 0x1a) continue; // CR_BUFFER_SMALL: device list changed between calls.
            if (result != 0) return null;
            return string.Join("\n", new string(buffer).Split(new[] { '\0' }, StringSplitOptions.RemoveEmptyEntries)
                .Select(id => id.ToUpperInvariant()).OrderBy(id => id, StringComparer.Ordinal));
        }
        return null;
    }
    private static void Reply(object packet)
    {
        Console.WriteLine(Json.Serialize(packet));
        Console.Out.Flush();
    }

    private static bool Wanted(ISensor s)
    {
        return s.SensorType == SensorType.Load || s.SensorType == SensorType.SmallData
            || s.SensorType == SensorType.Temperature || s.SensorType == SensorType.Clock;
    }

    private static object Metric(ISensor[] sensors, SensorType type, string name, bool failed, double scale)
    {
        var matches = sensors.Where(s => s.SensorType == type && s.Name == name).ToArray();
        if (failed) return new { status = "failed", value = (double?)null, detail = "GPU 读取失败" };
        if (matches.Length == 0)
            return new { status = "unsupported", value = (double?)null, detail = "此显卡未提供该指标" };
        if (matches.Length != 1)
            return new { status = "failed", value = (double?)null, detail = "驱动返回的指标不明确" };
        var value = matches[0].Value;
        return new { status = value.HasValue ? "normal" : "failed",
            value = value.HasValue ? (double?)(value.Value * scale) : null,
            detail = value.HasValue ? "" : "本次未读到有效数据" };
    }

    private static object Usage(ISensor[] sensors, bool warming, bool failed)
    {
        var engines = sensors.Where(s => s.SensorType == SensorType.Load && s.Name.StartsWith("D3D ")).ToArray();
        if (failed) return new { status = "failed", value = (double?)null, detail = "GPU 读取失败" };
        if (engines.Length == 0) return new { status = "unsupported", value = (double?)null, detail = "未提供 D3D 引擎占用" };
        if (warming) return new { status = "warming", value = (double?)null, detail = "正在建立 GPU 引擎采样基线" };
        if (engines.Any(s => !s.Value.HasValue)) return new { status = "failed", value = (double?)null, detail = "部分 GPU 引擎未返回读数" };
        return new { status = "normal", value = (double?)engines.Max(s => s.Value.Value), detail = "" };
    }

    public static int Main(string[] args)
    {
        int parentId;
        if (args.Length != 1 || !int.TryParse(args[0], out parentId)) return 2;
        Console.OutputEncoding = new System.Text.UTF8Encoding(false);
        Console.InputEncoding = new System.Text.UTF8Encoding(false);
        Process parent;
        try { parent = Process.GetProcessById(parentId); } catch { return 2; }
        var watcher = new Thread(() => {
            try { parent.WaitForExit(); } catch { }
            Environment.Exit(0);
        }) { IsBackground = true };
        watcher.Start();
        Computer computer = null;
        var inventoryAge = Stopwatch.StartNew();
        string inventory = DisplayDevices();
        var uptime = Stopwatch.StartNew();
        long previous = -1;
        bool first = true;
        try {
            while (Console.ReadLine() == "sample") {
                bool interrupted = previous >= 0 && uptime.ElapsedMilliseconds - previous > 3000;
                bool devicesChanged = false;
                if (inventoryAge.ElapsedMilliseconds >= 30000) {
                    string current = DisplayDevices();
                    if (current != null) {
                        devicesChanged = inventory != null && current != inventory;
                        inventory = current;
                    }
                    inventoryAge.Restart();
                }
                if (computer == null || devicesChanged || interrupted) {
                    if (computer != null) computer.Close();
                    Known.Clear();
                    computer = new Computer { IsGpuEnabled = true };
                    computer.Open();
                    first = true;
                }
                bool warming = first;
                var devices = new List<object>();
                foreach (var gpu in computer.Hardware.Where(h => h.HardwareType == HardwareType.GpuNvidia
                    || h.HardwareType == HardwareType.GpuAmd || h.HardwareType == HardwareType.GpuIntel).Take(16)) {
                    var property = gpu.GetType().GetProperty("DeviceId");
                    string deviceId = property == null ? null : property.GetValue(gpu, null) as string;
                    if (string.IsNullOrEmpty(deviceId)) continue;
                    foreach (var sensor in gpu.Sensors.Where(Wanted)) Known[sensor.Identifier.ToString()] = sensor;
                    foreach (var sensor in Known.Values.Where(s => s.Hardware == gpu)) {
                        sensor.ValuesTimeWindow = TimeSpan.Zero;
                        if (gpu.HardwareType == HardwareType.GpuAmd && sensor.SensorType == SensorType.SmallData
                            && sensor.Name == "GPU Memory Total") continue;
                        var valueProperty = sensor.GetType().GetProperty("Value");
                        if (valueProperty == null || !valueProperty.CanWrite) throw new InvalidOperationException("Unsupported LHM sensor cache");
                        valueProperty.SetValue(sensor, null, null);
                    }
                    bool failed = false;
                    try { gpu.Update(); } catch { failed = true; }
                    foreach (var sensor in gpu.Sensors.Where(Wanted)) Known[sensor.Identifier.ToString()] = sensor;
                    var sensors = Known.Values.Where(s => s.Hardware == gpu).ToArray();
                    devices.Add(new { id = deviceId.ToUpperInvariant(), name = gpu.Name, readings = new {
                        usage = Usage(sensors, warming, failed),
                        dedicated_used = Metric(sensors, SensorType.SmallData, "D3D Dedicated Memory Used", failed, 1048576),
                        shared_used = Metric(sensors, SensorType.SmallData, "D3D Shared Memory Used", failed, 1048576),
                        memory_total = Metric(sensors, SensorType.SmallData, "GPU Memory Total", failed, 1048576),
                        temperature = Metric(sensors, SensorType.Temperature, "GPU Core", failed, 1),
                        vr_soc_temperature = Metric(sensors, SensorType.Temperature, "GPU VR SoC", failed, 1),
                        core_clock = Metric(sensors, SensorType.Clock, "GPU Core", failed, 1),
                        memory_clock = Metric(sensors, SensorType.Clock, "GPU Memory", failed, 1)
                    }});
                }
                first = false;
                previous = uptime.ElapsedMilliseconds;
                Reply(new { status = "normal", devices });
            }
            return 0;
        } catch (UnauthorizedAccessException) {
            Reply(new { status = "permission_denied", detail = "GPU 驱动接口访问受限" });
            return 1;
        } catch {
            Reply(new { status = "failed", detail = "GPU 采集器读取失败" });
            return 1;
        } finally {
            if (computer != null) computer.Close();
            parent.Dispose();
        }
    }
}
