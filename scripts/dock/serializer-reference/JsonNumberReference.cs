// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Globalization;
using System.Text.Json;
using Dock.Serializer.SystemTextJson;

internal static class JsonNumberReference
{
    internal static void Run(string path)
    {
        var serializer = new DockSerializer();
        using var output = new StreamWriter(path);
        void Double(double value) => output.WriteLine($"write-f64\t{BitConverter.DoubleToUInt64Bits(value):x16}\t{serializer.Serialize(value)}");
        void Single(float value) => output.WriteLine($"write-f32\t{BitConverter.SingleToUInt32Bits(value):x8}\t{serializer.Serialize(value)}");
        foreach (double value in new[] { 0d, -0d, 1d, -1d, 0.1d, double.NaN, double.PositiveInfinity, double.NegativeInfinity, double.Epsilon, double.MaxValue, double.MinValue, Math.PI, 1e-5, 1e-4, 1e-3, 1e6, 1e7, 1e8, 1e9, 1e15, 1e16, 1e17, 1e20 }) Double(value);
        foreach (float value in new[] { 0f, -0f, 1f, -1f, 0.1f, float.NaN, float.PositiveInfinity, float.NegativeInfinity, float.Epsilon, float.MaxValue, float.MinValue, MathF.PI, 1e-5f, 1e-4f, 1e-3f, 1e6f, 1e7f, 1e8f, 1e9f, 1e15f, 1e16f, 1e17f, 1e20f }) Single(value);
        ulong state = 0xd0c0a11ce5eedUL;
        for (int i = 0; i < 512; i++)
        {
            state ^= state << 13; state ^= state >> 7; state ^= state << 17;
            Double(BitConverter.UInt64BitsToDouble(state));
            Single(BitConverter.UInt32BitsToSingle((uint)state));
        }
        foreach (string input in new[] { "0", "-0", "1", "1.0", "1e2", "1e400", "-1e400", "1e-400", "1e40", "1e-50", "3.4028235e38", "1.7976931348623157e308", "\"NaN\"", "\"Infinity\"", "\"-Infinity\"", "\"nan\"", "\"+Infinity\"", "\"1.5\"", "\"1\"", "\"\\u004eaN\"", "null", "true", "[]", "{}" })
        {
            string d, f;
            try { d = BitConverter.DoubleToUInt64Bits(serializer.Deserialize<double>(input)).ToString("x16"); } catch (JsonException) { d = "error"; }
            try { f = BitConverter.SingleToUInt32Bits(serializer.Deserialize<float>(input)).ToString("x8"); } catch (JsonException) { f = "error"; }
            output.WriteLine($"read-f64\t{input}\t{d}");
            output.WriteLine($"read-f32\t{input}\t{f}");
        }
        void Integer<T>(string name, T[] values)
        {
            foreach (T value in values) output.WriteLine($"write-{name}\t{Convert.ToString(value, CultureInfo.InvariantCulture)}\t{serializer.Serialize(value)}");
            foreach (string input in new[] { "0", "-0", "-1", "127", "128", "255", "256", "-128", "-129", "32767", "32768", "65535", "65536", "2147483647", "2147483648", "4294967295", "4294967296", "9223372036854775807", "9223372036854775808", "18446744073709551615", "18446744073709551616", "-9223372036854775808", "-9223372036854775809", "1.0", "1e2", "\"1\"", "null", "true" })
            {
                string result;
                try { result = Convert.ToString(serializer.Deserialize<T>(input), CultureInfo.InvariantCulture)!; } catch (JsonException) { result = "error"; }
                output.WriteLine($"read-{name}\t{input}\t{result}");
            }
        }
        Integer("i8", new[] { sbyte.MinValue, (sbyte)0, sbyte.MaxValue });
        Integer("u8", new[] { byte.MinValue, byte.MaxValue });
        Integer("i16", new[] { short.MinValue, (short)0, short.MaxValue });
        Integer("u16", new[] { ushort.MinValue, ushort.MaxValue });
        Integer("i32", new[] { int.MinValue, 0, int.MaxValue });
        Integer("u32", new[] { uint.MinValue, uint.MaxValue });
        Integer("i64", new[] { long.MinValue, 0L, long.MaxValue });
        Integer("u64", new[] { ulong.MinValue, ulong.MaxValue });
    }
}
