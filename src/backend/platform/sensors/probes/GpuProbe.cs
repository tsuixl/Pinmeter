using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Linq;
using System.Security.Principal;
using System.Threading;
using System.Web.Script.Serialization;
using LibreHardwareMonitor.Hardware;

// Standalone diagnostic, not part of the production CPU helper.
internal static class GpuProbe
{
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer();

    private static void Emit(object value)
    {
        Console.WriteLine(Json.Serialize(value));
        Console.Out.Flush();
    }

    private static string Now() { return DateTime.UtcNow.ToString("o"); }

    private static bool StaticInventory(ISensor sensor)
    {
        // AmdGpu 0.9.6 reads this once in its constructor and uses it to calculate free memory.
        return sensor.Hardware.HardwareType == HardwareType.GpuAmd
            && sensor.SensorType == SensorType.SmallData && sensor.Name == "GPU Memory Total";
    }

    private static bool Valid(ISensor sensor)
    {
        return sensor.Value.HasValue && !float.IsNaN(sensor.Value.Value) && !float.IsInfinity(sensor.Value.Value);
    }

    private static bool Wanted(ISensor sensor)
    {
        return sensor.SensorType == SensorType.Load || sensor.SensorType == SensorType.Clock
            || sensor.SensorType == SensorType.Temperature || sensor.SensorType == SensorType.SmallData
            || sensor.SensorType == SensorType.Data;
    }

    private static string Unit(ISensor sensor)
    {
        switch (sensor.SensorType) {
            case SensorType.Load: return "%";
            case SensorType.Clock: return "MHz";
            case SensorType.Temperature: return "C";
            case SensorType.SmallData: return "MiB";
            case SensorType.Data: return "GiB";
            default: return "unknown";
        }
    }

    private static object Reference()
    {
        string started = Now();
        var timer = Stopwatch.StartNew();
        try {
            using (var process = new Process()) {
                process.StartInfo = new ProcessStartInfo("nvidia-smi",
                    "--query-gpu=pci.bus_id,name,utilization.gpu,memory.used,memory.total,temperature.gpu,clocks.current.graphics,clocks.current.memory --format=csv") {
                    UseShellExecute = false, CreateNoWindow = true,
                    RedirectStandardOutput = true, RedirectStandardError = true
                };
                process.Start();
                var output = process.StandardOutput.ReadToEndAsync();
                var error = process.StandardError.ReadToEndAsync();
                if (!process.WaitForExit(3000)) {
                    process.Kill();
                    process.WaitForExit(1000);
                    return new { status = "timeout", started_at = started, completed_at = Now() };
                }
                return new { status = process.ExitCode == 0 ? "returned" : "failed",
                    started_at = started, completed_at = Now(), elapsed_ms = timer.Elapsed.TotalMilliseconds,
                    csv = output.Result, error = error.Result, exit_code = process.ExitCode };
            }
        } catch (Exception ex) {
            return new { status = "failed", started_at = started, completed_at = Now(), error = ex.Message };
        }
    }

    public static int Main(string[] args)
    {
        int count;
        if (args.Length != 1 || !int.TryParse(args[0], out count) || count < 2 || count > 60) return 2;
        Console.OutputEncoding = new System.Text.UTF8Encoding(false);
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        var computer = new Computer { IsGpuEnabled = true };
        var known = new Dictionary<string, ISensor>();
        var identity = WindowsIdentity.GetCurrent();
        Emit(new { kind = "environment", at = Now(), count,
            elevated = new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator),
            x64 = Environment.Is64BitProcess,
            lhm_version = typeof(Computer).Assembly.GetName().Version.ToString(),
            pawnio_installed = LibreHardwareMonitor.PawnIo.PawnIo.IsInstalled });
        try {
            var opening = Stopwatch.StartNew();
            computer.Open();
            Emit(new { kind = "opened", elapsed_ms = opening.Elapsed.TotalMilliseconds });
            var devices = computer.Hardware.Where(h => h.HardwareType == HardwareType.GpuNvidia
                || h.HardwareType == HardwareType.GpuAmd || h.HardwareType == HardwareType.GpuIntel).ToArray();
            if (devices.Length == 0) {
                Emit(new { kind = "error", error = "No GPU returned by LHM" });
                return 1;
            }
            for (int sample = 0; sample < count; sample++) {
                string started = Now();
                var timer = Stopwatch.StartNew();
                var rows = new List<object>();
                foreach (var gpu in devices) {
                    foreach (var sensor in gpu.Sensors.Where(Wanted)) known[sensor.Identifier.ToString()] = sensor;
                    var sensors = known.Values.Where(s => s.Hardware.Identifier.Equals(gpu.Identifier)).ToArray();
                    // Fixed LHM 0.9.6 uses an internal Sensor with a public Value setter.
                    foreach (var sensor in sensors) {
                        sensor.ValuesTimeWindow = TimeSpan.Zero;
                        if (StaticInventory(sensor)) continue;
                        var property = sensor.GetType().GetProperty("Value");
                        if (property == null || !property.CanWrite) throw new InvalidOperationException("Cannot clear LHM cache");
                        property.SetValue(sensor, null, null);
                    }
                    var update = Stopwatch.StartNew();
                    string failure = null;
                    try { gpu.Update(); } catch (Exception ex) { failure = ex.GetType().Name + ": " + ex.Message; }
                    double elapsed = update.Elapsed.TotalMilliseconds;
                    foreach (var sensor in gpu.Sensors.Where(Wanted)) known[sensor.Identifier.ToString()] = sensor;
                    var values = known.Values.Where(s => s.Hardware.Identifier.Equals(gpu.Identifier))
                        .OrderBy(s => s.Identifier.ToString()).Select(s => new {
                            id = s.Identifier.ToString(), name = s.Name, type = s.SensorType.ToString(), unit = Unit(s),
                            cache_policy = StaticInventory(s) ? "static_inventory" : (sensors.Contains(s) ? "cleared_before_update" : "newly_discovered"),
                            value = Valid(s) ? (float?)s.Value.Value : null,
                            status = failure != null ? "failed" : (sample == 0 && s.SensorType == SensorType.Load
                                ? "warmup" : (Valid(s) ? "returned" : "missing"))
                        }).ToArray();
                    var deviceProperty = gpu.GetType().GetProperty("DeviceId");
                    rows.Add(new { name = gpu.Name, id = gpu.Identifier.ToString(), type = gpu.HardwareType.ToString(),
                        device_id = deviceProperty == null ? null : deviceProperty.GetValue(gpu, null),
                        elapsed_ms = elapsed, error = failure, sensors = values });
                }
                double lhmMs = timer.Elapsed.TotalMilliseconds;
                using (var current = Process.GetCurrentProcess()) {
                    Emit(new { kind = "sample", index = sample, started_at = started, lhm_completed_at = Now(),
                        lhm_elapsed_ms = lhmMs, working_set_bytes = current.WorkingSet64,
                        cpu_ms = current.TotalProcessorTime.TotalMilliseconds, devices = rows, reference = Reference() });
                }
                if (sample + 1 < count) Thread.Sleep(1000);
            }
            return 0;
        } catch (Exception ex) {
            Emit(new { kind = "error", error = ex.ToString() });
            return 1;
        } finally {
            computer.Close();
            identity.Dispose();
            Emit(new { kind = "closed", at = Now() });
        }
    }
}
