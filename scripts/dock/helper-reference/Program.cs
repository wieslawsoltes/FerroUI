// Test-only compiler probe for the repeated Owner pattern in DockHelpers.
using System;

interface IRoot { }
sealed class Root : IRoot { }
sealed class Probe(object? value)
{
    public int Reads { get; private set; }
    public object? Owner
    {
        get { Reads++; return value; }
    }
}

static class Program
{
    static void Main()
    {
        foreach (object? value in new object?[] { new Root(), new object(), null })
        {
            Probe dockable = new(value);
            // Keep this pattern identical to the source shape: the compiler shares Owner.
            bool result = dockable is { Owner: not null, Owner: IRoot ownerRoot };
            if (result != (value is IRoot) || dockable.Reads != 1)
                throw new InvalidOperationException("Repeated Owner pattern semantics changed.");
            Console.WriteLine($"pattern={result},reads={dockable.Reads}");
        }
    }
}
