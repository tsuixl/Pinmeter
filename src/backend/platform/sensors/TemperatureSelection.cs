using System;
using System.Linq;

internal sealed class TemperatureSensor
{
    public string Name;
    public float? Value;
    public TemperatureSensor(string name, float? value) { Name = name; Value = value; }
}

internal static class TemperatureSelection
{
    // Tctl alone may contain an offset; prefer actual die/package temperature.
    internal static double? Select(TemperatureSensor[] sensors)
    {
        foreach (string name in new[] { "CPU Package", "Core (Tdie)", "Core (Tctl/Tdie)" }) {
            var sensor = sensors.FirstOrDefault(s => s.Name == name);
            if (sensor != null) return Valid(sensor.Value);
        }
        var cores = sensors.Where(s => s.Name.StartsWith("CPU Core #", StringComparison.Ordinal)
            && !s.Name.Contains("Distance")).ToArray();
        if (cores.Length == 0 || cores.Any(s => !Valid(s.Value).HasValue)) return null;
        return cores.Max(s => (double)s.Value.Value);
    }

    private static double? Valid(float? value)
    {
        return value.HasValue && !float.IsNaN(value.Value) && !float.IsInfinity(value.Value)
            && value.Value >= -50 && value.Value <= 150 ? (double?)value.Value : null;
    }
}
