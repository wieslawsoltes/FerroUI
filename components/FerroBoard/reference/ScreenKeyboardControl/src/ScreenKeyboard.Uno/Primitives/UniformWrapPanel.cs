using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Foundation;

namespace ScreenKeyboard.Controls.Primitives;

/// <summary>A panel arranging children in a grid of equally sized cells that fill the available width.</summary>
public sealed partial class UniformWrapPanel : Panel
{
    /// <summary>Desired (minimum) cell width.</summary>
    public double ItemWidth { get; set; } = 44;

    /// <summary>Cell height.</summary>
    public double ItemHeight { get; set; } = 44;

    /// <summary>Maximum number of columns (0 = unlimited).</summary>
    public int MaxColumns { get; set; }

    private int Columns(double width)
    {
        var columns = double.IsInfinity(width) || width <= 0 ? 8 : Math.Max(1, (int)Math.Floor(width / ItemWidth));
        return MaxColumns > 0 ? Math.Min(columns, MaxColumns) : columns;
    }

    /// <inheritdoc />
    protected override Size MeasureOverride(Size availableSize)
    {
        var columns = Columns(availableSize.Width);
        var cellWidth = double.IsInfinity(availableSize.Width) ? ItemWidth : availableSize.Width / columns;
        foreach (var child in Children)
        {
            child.Measure(new Size(cellWidth, ItemHeight));
        }
        var rows = (int)Math.Ceiling(Children.Count / (double)columns);
        var width = double.IsInfinity(availableSize.Width) ? columns * ItemWidth : availableSize.Width;
        return new Size(width, rows * ItemHeight);
    }

    /// <inheritdoc />
    protected override Size ArrangeOverride(Size finalSize)
    {
        var columns = Columns(finalSize.Width);
        var cellWidth = finalSize.Width / columns;
        for (var i = 0; i < Children.Count; i++)
        {
            var row = i / columns;
            var col = i % columns;
            Children[i].Arrange(new Rect(col * cellWidth, row * ItemHeight, cellWidth, ItemHeight));
        }
        var rows = (int)Math.Ceiling(Children.Count / (double)columns);
        return new Size(finalSize.Width, Math.Max(finalSize.Height, rows * ItemHeight));
    }
}
