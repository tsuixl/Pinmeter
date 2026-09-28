using System;
internal static class SelectionTests
{
    static TemperatureSensor S(string name, float? value) { return new TemperatureSensor(name, value); }
    static void Check(double? actual, double? expected) { if (actual != expected) throw new Exception("Temperature selection mismatch"); }
    static void Main()
    {
        Check(TemperatureSelection.Select(new[] { S("CPU Package", 60), S("CPU Core #1", 80) }), 60);
        Check(TemperatureSelection.Select(new[] { S("Core (Tctl)", 90), S("Core (Tdie)", 70) }), 70);
        Check(TemperatureSelection.Select(new[] { S("Core (Tctl/Tdie)", 65) }), 65);
        Check(TemperatureSelection.Select(new[] { S("CPU Package", null), S("CPU Core #1", 80) }), null);
        Check(TemperatureSelection.Select(new[] { S("CPU Core #1", 40), S("CPU Core #2", 50) }), 50);
        Check(TemperatureSelection.Select(new[] { S("CPU Core #1", 40), S("CPU Core #2", null) }), null);
        Check(TemperatureSelection.Select(new[] { S("CPU Package", 0) }), 0);
        Check(TemperatureSelection.Select(new[] { S("CPU Package", float.NaN) }), null);
        Check(TemperatureSelection.Select(new[] { S("CPU Package", 200) }), null);
        Check(TemperatureSelection.Select(new[] { S("ACPI Thermal Zone", 50), S("Core (Tctl)", 90) }), null);
        Console.WriteLine("PASS: package/die preference, core maximum, missing, zero, invalid and unrelated sensors");
    }
}
