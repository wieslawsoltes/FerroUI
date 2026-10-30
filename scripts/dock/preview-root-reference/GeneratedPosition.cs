// Generated from the frozen source GetManagedPosition body by generate.py.
// The parameter uses a Visual alias because the body reads only Visual APIs.
using Avalonia;
using Avalonia.Controls;
using Avalonia.VisualTree;
using ManagedWindowLayer = Avalonia.Visual;
partial class Program
{
    private static Point GetManagedPosition(ManagedWindowLayer layer, PixelPoint position, PixelPoint offset)
    {
        if (layer.GetVisualRoot() is not TopLevel topLevel)
        {
            return new Point(0, 0);
        }

        var target = new PixelPoint(position.X + offset.X, position.Y + offset.Y);
        var clientPoint = topLevel.PointToClient(target);
        var layerOrigin = layer.TranslatePoint(new Point(0, 0), topLevel) ?? new Point(0, 0);
        var local = new Point(clientPoint.X - layerOrigin.X, clientPoint.Y - layerOrigin.Y);
        return local;
    }
}
