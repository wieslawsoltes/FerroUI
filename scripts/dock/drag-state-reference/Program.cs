// Numeric reference for the two expressions used by DockControlState.
using System;
using System.Globalization;

CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
Console.WriteLine("input_bits\trounded_pixel\tmax_zero_bits");
foreach (double value in new[]
{
    double.NaN, double.NegativeInfinity, double.PositiveInfinity, -0.0, 0.0,
    0.5, -0.5, 1.5, -1.5, 2.5, -2.5,
    2147483647.0, 2147483647.5, 2147483648.0,
    -2147483648.0, -2147483648.5, -2147483649.0
})
{
    // Keep these expressions identical to their source shape, including the default rounding mode.
    int pixel = unchecked((int)Math.Round(value));
    double dimension = Math.Max(0.0, value);
    Console.WriteLine($"{BitConverter.DoubleToUInt64Bits(value):x16}\t{pixel}\t{BitConverter.DoubleToUInt64Bits(dimension):x16}");
}
