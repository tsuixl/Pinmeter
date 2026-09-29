using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.Reflection;
using System.Web.Script.Serialization;
using LibreHardwareMonitor.Hardware;
using LibreHardwareMonitor.PawnIo;
using Microsoft.Win32.SafeHandles;

internal static class Program
{
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFile(string name, uint access, uint share,
        IntPtr security, uint disposition, uint flags, IntPtr template);
    private static void Reply(string status, double? value, string detail)
    {
        Console.WriteLine(new JavaScriptSerializer().Serialize(new { status, value, detail }));
        Console.Out.Flush();
    }

    public static int Main(string[] args)
    {
        Console.OutputEncoding = new System.Text.UTF8Encoding(false);
        Console.InputEncoding = new System.Text.UTF8Encoding(false);
        if (args.Length != 1) return 2;
        if (args[0] == "--pawnio-status") return PawnIODriver.Status();
        Process parent;
        try { parent = Process.GetProcessById(int.Parse(args[0], CultureInfo.InvariantCulture)); }
        catch { return 2; }
        var watcher = new Thread(() => {
            while (true) {
                if (parent.HasExited) Environment.Exit(0);
                Thread.Sleep(500);
            }
        });
        watcher.IsBackground = true;
        watcher.Start();
        Computer computer = null;
        try {
            while (Console.ReadLine() == "sample") {
                if (computer == null) {
                    if (!PawnIo.IsInstalled) {
                        Reply("unsupported", null, "未安装 CPU 温度驱动（PawnIO）");
                        return 0;
                    }
                    using (var device = CreateFile(@"\\?\GLOBALROOT\Device\PawnIO", 0xC0000000,
                        3, IntPtr.Zero, 3, 0, IntPtr.Zero)) {
                        if (device.IsInvalid) {
                            int error = Marshal.GetLastWin32Error();
                            Reply(error == 5 ? "permission_denied" : "failed", null,
                                error == 5 ? "没有 CPU 温度传感器访问权限" : "PawnIO 驱动未加载或无法访问");
                            return 0;
                        }
                    }
                    computer = new Computer { IsCpuEnabled = true };
                    computer.Open();
                }
                var values = new List<double>();
                foreach (var cpu in computer.Hardware.Where(h => h.HardwareType == HardwareType.Cpu)) {
                    foreach (var sensor in cpu.Sensors) {
                        sensor.ValuesTimeWindow = TimeSpan.Zero;
                        if (sensor.SensorType == SensorType.Temperature) {
                            // Fixed LHM 0.9.6 Sensor has a public setter on an internal type.
                            // Clear its cached value so a skipped/failed update cannot look fresh.
                            var property = sensor.GetType().GetProperty("Value");
                            if (property == null || !property.CanWrite) throw new InvalidOperationException();
                            property.SetValue(sensor, null, null);
                        }
                    }
                    cpu.Update();
                    var sensors = cpu.Sensors.Where(s => s.SensorType == SensorType.Temperature)
                        .Select(s => new TemperatureSensor(s.Name, s.Value)).ToArray();
                    double? selected = TemperatureSelection.Select(sensors);
                    if (!selected.HasValue) {
                        Reply("failed", null, "CPU 封装或核心温度传感器没有有效读数");
                        return 0;
                    }
                    values.Add(selected.Value);
                }
                if (values.Count == 0) {
                    Reply("unsupported", null, "没有受支持的 CPU 温度传感器");
                    return 0;
                }
                Reply("normal", values.Max(), "");
            }
            return 0;
        }
        catch (UnauthorizedAccessException) { Reply("permission_denied", null, "CPU 温度采集权限不足"); return 1; }
        catch (Exception) { Reply("failed", null, "CPU 温度采集失败"); return 1; }
        finally { if (computer != null) computer.Close(); parent.Dispose(); }
    }
}
